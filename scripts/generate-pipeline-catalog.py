"""Generate the checked-in, network-independent v2 contract catalogue."""
import json
from pathlib import Path

S = {"type": "string"}
ID = {"type": "string", "minLength": 1}
N = {"type": "integer", "minimum": 0}
F = {"type": "number"}
B = {"type": "boolean"}
def array(item): return {"type": "array", "items": item}
def obj(**properties):
    return {"type": "object", "properties": properties, "required": list(properties), "additionalProperties": False}
RANGE = obj(segment_id=ID, start_utf8=N, end_utf8=N)
TOKEN = obj(token_id=ID, segment_id=ID, text=S, start_utf8=N, end_utf8=N)
SEG = obj(segment_id=ID, text=S, language=S)
CONTEXT = obj(context_id=ID, sources=array(SEG), targets=array(SEG))
OCC = obj(occurrence_id=ID, source=RANGE, matched_ranges=array(RANGE), text=S)
PAIR = obj(pair_id=ID, left=S, right=S)
EVIDENCE = obj(pair_id=ID, score=F, score_kind=ID, provider_id=ID)
QUERY = obj(text=S, fuzzy=B, max_gap=N, similarity_operator=ID)
QUERY["properties"]["minimum_similarity"] = {"type":"number","exclusiveMinimum":0,"maximum":1}
schemas = {
    "SourceAsset": obj(asset_id=ID, media_type=S, text=S),
    "ClipboardSnapshot": obj(snapshot_id=ID, text=S),
    "ExtractedText": obj(asset_id=ID, text=S, language=S),
    "SegmentProposalSet": obj(asset_id=ID, segments=array(obj(text=S, start_utf8=N, end_utf8=N))),
    "ManualAlignmentIntent": obj(source_segment_ids=array(ID), target_segment_ids=array(ID), replace_existing=B),
    "CanonicalView": obj(project_id=ID, revision_id=ID, segments=array(obj(segment_id=ID,text=S,language=S,side={"enum":["source","target"]})), alignments=array(obj(alignment_id=ID, source_segment_ids=array(ID), target_segment_ids=array(ID)))),
    "KernelCommandProposal": obj(proposal_id=ID, kind={"enum":["link_segments"]}, source_segment_ids=array(ID), target_segment_ids=array(ID), replace_existing=B),
    "SegmentView": obj(segments=array(SEG)),
    "TextView": obj(segments=array(SEG)),
    "QuerySpec": QUERY,
    "OccurrenceSet": obj(occurrences=array(OCC)),
    "ExportSpec": obj(side={"enum":["source","target","parallel"]}),
    "ExportArtifact": obj(media_type=S, content=S),
    "TimedTextRegions": obj(asset_id=ID, regions=array(obj(region_id=ID, text=S, start_ms=N, end_ms=N))),
    "SpatialTextRegions": obj(asset_id=ID, regions=array(obj(region_id=ID, text=S, page=N, x=F, y=F, width=F, height=F))),
    "RegionOrderProposal": obj(asset_id=ID, ordered_region_ids=array(ID)),
    "LinguisticTokenBatch": obj(tokenizer_id=ID, tokens=array(TOKEN)),
    "TextMap": obj(coordinate_unit={"const":"utf8_bytes"}, ranges=array(RANGE)),
    "SegmentEmbeddingBatch": obj(model_id=ID, dimensions=N, items=array(obj(segment_id=ID, vector=array(F)))),
    "TranslationProposalSet": obj(items=array(obj(segment_id=ID, text=S, language=S))),
    "TokenAnnotationBatch": obj(items=array(obj(token_id=ID, label=S))),
    "EntitySpanBatch": obj(items=array(obj(ranges=array(RANGE), entity_type=S))),
    "DependencyGraphBatch": obj(edges=array(obj(head_token_id=ID, dependent_token_id=ID, relation=S))),
    "AlignmentProposalSet": obj(items=array(obj(source_segment_ids=array(ID), target_segment_ids=array(ID), score=F))),
    "ParallelContextSet": obj(contexts=array(CONTEXT)),
    "WordAlignmentArtifact": obj(contexts=array(obj(context_id=ID, coverage={"enum":["complete","partial","no_links"]}, edges=array(obj(source=RANGE, target=RANGE, score=F)), score_kind=ID, provider_id=ID))),
    "TerminologyCandidateSet": obj(items=array(obj(source=S, target=S, evidence_ranges=array(RANGE)))),
    "BasicIndexRef": obj(index_id=ID, items=array(obj(segment_id=ID, text=S, language=S, normalized_text=S)), postings=array(obj(term=S,segment_ids=array(ID)))),
    "LexicalIndexRef": obj(index_id=ID, tokenizer_id=ID, tokens=array(TOKEN), postings=array(obj(term=S,occurrences=array(obj(token_id=ID,segment_id=ID,start_utf8=N,end_utf8=N,position=N))))),
    "VectorIndexRef": obj(index_id=ID, model_id=ID, dimensions=N, segment_ids=array(ID)),
    "WindowSpec": obj(before=N, after=N),
    "KwicView": obj(items=array(obj(occurrence_id=ID, segment_id=ID, before=S, matched=S, after=S, ranges=array(RANGE)))),
    "AnalysisSpec": obj(window=N, limit=N),
    "CollocationResult": obj(items=array(obj(left=S, right=S, count=N, score=F))),
    "TermFrequencyResult": obj(items=array(obj(term=S, count=N))),
    "QualityIssueSet": obj(items=array(obj(issue_id=ID, segment_ids=array(ID), code=S, message=S))),
    "TextPairBatch": obj(pairs=array(PAIR)),
    "SimilarityEvidenceBatch": obj(items=array(EVIDENCE)),
    "TranslationCandidateSet": obj(items=array(obj(occurrence_id=ID, candidates=array(obj(ranges=array(RANGE), text=S, score=F, score_kind=ID, provider_id=ID))))),
    "ConfirmedJudgementView": obj(items=array(obj(record_id=ID, group_name=S, strategy=S, text=S))),
    "GroupSuggestionSet": obj(items=array(obj(left_group=S, right_group=S, score=F, reason=S))),
}
schemas["TranslationCandidateSet"]["properties"]["items"]["items"]["properties"]["alignment_coverage"] = {"enum":["complete","partial","no_links","context_missing"]}
def ports(spec):
    return [{"name": name, "schemas": [{"name": schema, "version": 1} for schema in types.split("|")], "required": True, "multiple": False} for name,types in spec]
rows = [
 ("source.text.parse", [("asset","SourceAsset")], [("text","ExtractedText")]),
 ("source.clipboard", [("clipboard","ClipboardSnapshot")], [("text","ExtractedText")]),
 ("content.segment.sentence", [("text","ExtractedText")], [("segments","SegmentProposalSet")]),
 ("relation.alignment.manual", [("intent","ManualAlignmentIntent"),("view","CanonicalView")], [("proposal","KernelCommandProposal")]),
 ("index.basic_string", [("segments","SegmentView")], [("index","BasicIndexRef")]),
 ("analysis.basic_search", [("query","QuerySpec"),("segments","SegmentView|BasicIndexRef")], [("occurrences","OccurrenceSet")]),
 *[("export."+fmt,[("view","CanonicalView"),("spec","ExportSpec")],[("export","ExportArtifact")]) for fmt in ("txt","json","xml")],
 ("source.subtitle.parse", [("asset","SourceAsset")], [("regions","TimedTextRegions")]),
 ("source.ocr", [("asset","SourceAsset")], [("regions","SpatialTextRegions")]),
 ("source.audio.transcribe", [("asset","SourceAsset")], [("regions","TimedTextRegions")]),
 ("layout.reading_order", [("regions","SpatialTextRegions")], [("order","RegionOrderProposal")]),
 ("segment.tokenize", [("segments","SegmentView")], [("tokens","LinguisticTokenBatch"),("map","TextMap")]),
 ("segment.embedding", [("segments","SegmentView")], [("embeddings","SegmentEmbeddingBatch")]),
 ("segment.translate", [("segments","SegmentView")], [("translations","TranslationProposalSet")]),
 *[("token."+name,[("tokens","LinguisticTokenBatch")],[("annotations",output)]) for name,output in [("pos","TokenAnnotationBatch"),("lemma","TokenAnnotationBatch"),("ner","EntitySpanBatch"),("dependency","DependencyGraphBatch")]],
 ("relation.alignment.auto", [("source","SegmentView"),("target","SegmentView")], [("alignments","AlignmentProposalSet")]),
 ("relation.word_alignment", [("contexts","ParallelContextSet")], [("alignments","WordAlignmentArtifact")]),
 ("relation.terminology", [("contexts","ParallelContextSet")], [("terms","TerminologyCandidateSet")]),
 ("index.lexical", [("tokens","LinguisticTokenBatch")], [("index","LexicalIndexRef")]),
 ("index.vector", [("embeddings","SegmentEmbeddingBatch")], [("index","VectorIndexRef")]),
 ("analysis.kwic", [("occurrences","OccurrenceSet"),("text","TextView"),("window","WindowSpec")], [("kwic","KwicView")]),
 ("analysis.collocation", [("tokens","LinguisticTokenBatch"),("spec","AnalysisSpec")], [("collocations","CollocationResult")]),
 ("analysis.wordcloud", [("tokens","LinguisticTokenBatch"),("spec","AnalysisSpec")], [("terms","TermFrequencyResult")]),
 ("analysis.quality", [("view","CanonicalView")], [("issues","QualityIssueSet")]),
 ("text.similarity", [("pairs","TextPairBatch")], [("scores","SimilarityEvidenceBatch")]),
 ("analysis.fuzzy_search", [("query","QuerySpec"),("segments","SegmentView|BasicIndexRef")], [("occurrences","OccurrenceSet")]),
 ("analysis.translation_candidates", [("occurrences","OccurrenceSet"),("contexts","ParallelContextSet"),("alignments","WordAlignmentArtifact")], [("candidates","TranslationCandidateSet")]),
 ("analysis.translation_grouping", [("judgements","ConfirmedJudgementView")], [("suggestions","GroupSuggestionSet")]),
]
catalog = {"schemas": [{"name": name, "version":1, "schema": {"$schema":"https://json-schema.org/draft/2020-12/schema", **schema}} for name,schema in schemas.items()], "slots":[{"slot_id":slot,"name":slot,"state":"unbound","reason":"没有可用的算法实现","inputs":ports(inputs),"outputs":ports(outputs),"providers":[]} for slot,inputs,outputs in rows]}
target=Path(__file__).resolve().parents[1]/"schemas/pipeline/catalog-v2.json"
target.parent.mkdir(parents=True,exist_ok=True)
target.write_text(json.dumps(catalog,ensure_ascii=False,indent=2)+"\n")
