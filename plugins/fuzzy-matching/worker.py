"""Jueming local worker protocol 1. Stdout is exclusively bounded JSON frames."""
import json
import sys
import unicodedata
from collections import Counter

MAX_FRAME = 8 * 1024 * 1024
OPERATORS = ["fuzzy.edit_distance", "fuzzy.char_ngram"]

def normalize(text):
    return unicodedata.normalize("NFKC", text).casefold()

def edit_distance(a, b):
    if len(a) > len(b):
        a, b = b, a
    row = list(range(len(a) + 1))
    for j, right in enumerate(b, 1):
        next_row = [j]
        for i, left in enumerate(a, 1):
            next_row.append(min(next_row[-1] + 1, row[i] + 1, row[i-1] + (left != right)))
        row = next_row
    return 1.0 - row[-1] / max(len(a), len(b), 1)

def char_ngram(a, b):
    if a == b:
        return 1.0
    def grams(s):
        s = "\x02" + s + "\x03"
        return Counter(s[i:i+2] for i in range(len(s)-1))
    x, y = grams(a), grams(b)
    return 2 * sum((x & y).values()) / max(sum(x.values()) + sum(y.values()), 1)

def execute(request):
    operator = request["operator_id"]
    if operator not in OPERATORS:
        raise ValueError("unregistered operator")
    pairs = request["inputs"]["pairs"][0]["pairs"]
    if len(pairs) > 2048:
        raise ValueError("split pairs into batches of at most 2048")
    score = edit_distance if operator == OPERATORS[0] else char_ngram
    items = []
    for pair in pairs:
        a, b = normalize(pair["left"]), normalize(pair["right"])
        if max(len(a), len(b)) > 512:
            raise ValueError("comparison unit exceeds 512 characters")
        items.append({"pair_id":pair["pair_id"], "score":score(a,b), "score_kind":"normalized_edit_similarity" if operator == OPERATORS[0] else "bigram_dice", "provider_id":operator})
    return {"scores":{"items":items}}

def main():
    while True:
        line = sys.stdin.buffer.readline(MAX_FRAME + 1)
        if not line:
            return
        if len(line) > MAX_FRAME or not line.endswith(b"\n"):
            return
        request = json.loads(line)
        try:
            data = {"protocol_version":1,"operators":OPERATORS} if request["method"] == "hello" else execute(request)
            response = {"request_id":request["request_id"], "data":data}
        except Exception as error:
            response = {"request_id":request.get("request_id"), "error":str(error)}
        encoded = json.dumps(response,ensure_ascii=False).encode()
        if len(encoded) > MAX_FRAME:
            encoded = json.dumps({"request_id":request["request_id"],"error":"output frame limit exceeded"}).encode()
        sys.stdout.buffer.write(encoded+b"\n")
        sys.stdout.buffer.flush()

if __name__ == "__main__":
    main()
