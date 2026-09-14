"""Contextual XLM-R mutual-nearest subword alignment with original UTF-8 anchors.

Similarity is uncalibrated evidence, never a correctness probability. Empty links
mean no evidence; the human must decide omission versus paraphrase.
"""
import json
import os
import sys

MAX_FRAME = 8 * 1024 * 1024
OPERATOR = "xlmr.contextual_alignment"
model = tokenizer = torch = None

def load_model():
    global model, tokenizer, torch
    if model is not None:
        return
    import torch as torch_module
    from transformers import AutoModel, AutoTokenizer
    torch = torch_module
    torch.set_num_threads(max(1, min(4, os.cpu_count() or 1)))
    directory = os.environ["JUEMING_MODEL_DIR"]
    tokenizer = AutoTokenizer.from_pretrained(directory, local_files_only=True, use_fast=True)
    model = AutoModel.from_pretrained(directory, local_files_only=True, use_safetensors=True)
    model.eval()

def embed(segment):
    text = segment["text"]
    data = tokenizer(text, return_offsets_mapping=True, return_tensors="pt", truncation=True, max_length=512)
    offsets = data.pop("offset_mapping")[0].tolist()
    with torch.inference_mode():
        hidden = model(**data, output_hidden_states=True).hidden_states[8][0]
    indices = [i for i,(start,end) in enumerate(offsets) if end > start]
    vectors = torch.nn.functional.normalize(hidden[indices], p=2, dim=-1)
    ranges = [{"segment_id":segment["segment_id"],"start_utf8":len(text[:offsets[i][0]].encode()),"end_utf8":len(text[:offsets[i][1]].encode())} for i in indices]
    complete = not indices or offsets[indices[-1]][1] >= len(text.rstrip())
    return vectors, ranges, complete

def execute(request):
    if request["operator_id"] != OPERATOR:
        raise ValueError("unregistered operator")
    contexts = request["inputs"]["contexts"][0]["contexts"]
    if len(contexts) > 32:
        raise ValueError("split contexts into batches of at most 32")
    load_model()
    result = []
    cache = {}
    for context in contexts:
        edges, complete = [], True
        if len(context["sources"]) + len(context["targets"]) > 64:
            raise ValueError("context exceeds segment limit")
        for source in context["sources"]:
            if source["segment_id"] not in cache:
                cache[source["segment_id"]] = embed(source)
            sv, sr, sc = cache[source["segment_id"]]
            complete &= sc
            for target in context["targets"]:
                if target["segment_id"] not in cache:
                    cache[target["segment_id"]] = embed(target)
                tv, tr, tc = cache[target["segment_id"]]
                complete &= tc
                if not sr or not tr:
                    continue
                similarities = sv @ tv.T
                forward, backward = similarities.argmax(1), similarities.argmax(0)
                for i, j in enumerate(forward.tolist()):
                    score = float(similarities[i,j])
                    if int(backward[j]) == i and score >= 0.15:
                        edges.append({"source":sr[i],"target":tr[j],"score":score})
        result.append({"context_id":context["context_id"],"coverage":"partial" if not complete else "complete" if edges else "no_links","edges":edges,"score_kind":"contextual_cosine_layer8_mutual_nearest","provider_id":OPERATOR})
    return {"alignments":{"contexts":result}}

def main():
    while True:
        line = sys.stdin.buffer.readline(MAX_FRAME + 1)
        if not line:
            return
        if len(line) > MAX_FRAME or not line.endswith(b"\n"):
            return
        request = json.loads(line)
        try:
            if request["method"] == "hello":
                # Readiness includes a real model load, not just successful spawn.
                load_model()
                data = {"protocol_version":1,"operators":[OPERATOR]}
            else:
                data = execute(request)
            response = {"request_id":request["request_id"],"data":data}
        except Exception as error:
            response = {"request_id":request.get("request_id"),"error":str(error)}
        encoded = json.dumps(response,ensure_ascii=False).encode()
        if len(encoded) > MAX_FRAME:
            encoded=json.dumps({"request_id":request["request_id"],"error":"output frame limit exceeded"}).encode()
        sys.stdout.buffer.write(encoded+b"\n")
        sys.stdout.buffer.flush()

if __name__ == "__main__":
    main()
