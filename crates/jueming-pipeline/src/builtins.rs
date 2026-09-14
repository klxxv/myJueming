//! Pure adapters. Filesystem saves and canonical mutations remain Host commands.
use crate::{
    executor::{Provider, Values},
    registry::Registry,
};
use jueming_protocol::OperatorDescriptor;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicBool, Ordering},
};

pub const BUILTINS: &[(&str, &str)] = &[
    ("source.text.parse", "builtin.text_parse"),
    ("source.clipboard", "builtin.clipboard"),
    ("content.segment.sentence", "builtin.sentences"),
    ("relation.alignment.manual", "builtin.manual_proposal"),
    ("index.basic_string", "builtin.basic_index"),
    ("analysis.basic_search", "builtin.search"),
    ("export.txt", "builtin.export_txt"),
    ("export.json", "builtin.export_json"),
    ("export.xml", "builtin.export_xml"),
    ("segment.tokenize", "builtin.tokenize"),
    ("index.lexical", "builtin.lexical_index"),
    ("analysis.kwic", "builtin.kwic"),
    ("analysis.fuzzy_search", "builtin.fuzzy_search"),
    (
        "analysis.translation_candidates",
        "builtin.translation_candidates",
    ),
    (
        "analysis.translation_grouping",
        "builtin.translation_grouping",
    ),
];
pub fn bind(registry: &mut Registry, slot: &str, id: &str) -> Result<(), String> {
    let slot = registry.slot(slot).ok_or("unknown slot")?.clone();
    registry.register_operator(OperatorDescriptor {
        operator_id: id.into(),
        release: "1.0.0".into(),
        name: id.into(),
        slots: vec![slot.slot_id],
        inputs: slot.inputs,
        outputs: slot.outputs,
        config_schema: json!({"type":"object","maxProperties":0}),
    })
}
pub fn register(registry: &mut Registry) -> Result<(), String> {
    registry.register_input_adapters()?;
    for (slot, id) in BUILTINS {
        bind(registry, slot, id)?;
    }
    Ok(())
}
fn input<'a>(inputs: &'a Values, name: &str) -> Result<&'a Value, String> {
    inputs
        .get(name)
        .and_then(|v| v.first())
        .ok_or_else(|| format!("missing input {name}"))
}
fn items<'a>(value: &'a Value, name: &str) -> Result<&'a Vec<Value>, String> {
    value[name]
        .as_array()
        .ok_or_else(|| format!("missing array {name}"))
}
fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .ok_or_else(|| format!("missing text {name}"))
}
fn output(port: &str, value: Value) -> BTreeMap<String, Value> {
    BTreeMap::from([(port.into(), value)])
}
fn id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// UTF-8 coordinates are always against original content. Normalization never shifts anchors.
pub fn word_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (offset, c) in text.char_indices() {
        let cjk = ('\u{3400}'..='\u{9fff}').contains(&c);
        if c.is_alphanumeric() && !cjk {
            if start.is_none() {
                start = Some(offset);
            }
        } else {
            if let Some(s) = start.take() {
                ranges.push((s, offset));
            }
            if cjk {
                ranges.push((offset, offset + c.len_utf8()));
            }
        }
    }
    if let Some(s) = start {
        ranges.push((s, text.len()));
    }
    ranges
}
pub struct Builtins<'a> {
    pub plugins: &'a dyn Provider,
}
pub fn default_similarity_threshold(operator: &str) -> Option<f64> {
    match operator {
        "fuzzy.edit_distance" => Some(0.75),
        "fuzzy.char_ngram" => Some(0.65),
        _ => None,
    }
}
impl Provider for Builtins<'_> {
    fn execute(
        &self,
        operator: &str,
        inputs: Values,
        config: &Value,
        cancelled: &AtomicBool,
    ) -> Result<BTreeMap<String, Value>, String> {
        if cancelled.load(Ordering::Acquire) {
            return Err("cancelled".into());
        }
        match operator {
            "builtin.text_parse" | "builtin.clipboard" => {
                let v = input(
                    &inputs,
                    if operator == "builtin.clipboard" {
                        "clipboard"
                    } else {
                        "asset"
                    },
                )?;
                Ok(output(
                    "text",
                    json!({"asset_id":v.get("asset_id").or_else(||v.get("snapshot_id")),"text":v["text"],"language":"und"}),
                ))
            }
            "builtin.sentences" => {
                let v = input(&inputs, "text")?;
                let t = text(v, "text")?;
                let mut result = Vec::new();
                let mut start = 0;
                for (i, c) in t.char_indices() {
                    if matches!(c, '。' | '！' | '？' | '\n' | '.' | '!' | '?') {
                        let end = i + c.len_utf8();
                        if !t[start..end].trim().is_empty() {
                            result.push(
                                json!({"text":&t[start..end],"start_utf8":start,"end_utf8":end}),
                            );
                        }
                        start = end;
                    }
                }
                if start < t.len() && !t[start..].trim().is_empty() {
                    result.push(json!({"text":&t[start..],"start_utf8":start,"end_utf8":t.len()}));
                }
                Ok(output(
                    "segments",
                    json!({"asset_id":v["asset_id"],"segments":result}),
                ))
            }
            "builtin.manual_proposal" => {
                let intent = input(&inputs, "intent")?;
                let view = input(&inputs, "view")?;
                for name in ["source_segment_ids", "target_segment_ids"] {
                    for segment in items(intent, name)? {
                        if !items(view, "segments")?
                            .iter()
                            .any(|s| &s["segment_id"] == segment)
                        {
                            return Err("proposal references absent Segment".into());
                        }
                    }
                }
                Ok(output(
                    "proposal",
                    json!({"proposal_id":id(),"kind":"link_segments","source_segment_ids":intent["source_segment_ids"],"target_segment_ids":intent["target_segment_ids"],"replace_existing":intent["replace_existing"]}),
                ))
            }
            "builtin.basic_index" => {
                let mut postings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
                let mut documents = Vec::new();
                for segment in items(input(&inputs, "segments")?, "segments")? {
                    let content = text(segment, "text")?;
                    let segment_id = text(segment, "segment_id")?;
                    for (start, end) in word_ranges(content) {
                        postings
                            .entry(content[start..end].to_lowercase())
                            .or_default()
                            .insert(segment_id.into());
                    }
                    documents.push(json!({"segment_id":segment_id,"text":content,"language":segment["language"],"normalized_text":content.to_lowercase()}));
                }
                Ok(output(
                    "index",
                    json!({"index_id":id(),"items":documents,"postings":postings.into_iter().map(|(term,segment_ids)|json!({"term":term,"segment_ids":segment_ids})).collect::<Vec<_>>()}),
                ))
            }
            "builtin.tokenize" => {
                let mut tokens = Vec::new();
                let mut ranges = Vec::new();
                let jieba = jieba_rs::Jieba::new();
                for s in items(input(&inputs, "segments")?, "segments")? {
                    let content = text(s, "text")?;
                    let mut offset = 0;
                    for token in jieba.cut(content, false) {
                        let start = offset;
                        offset += token.word.len();
                        if token.word.trim().is_empty() {
                            continue;
                        }
                        tokens.push(json!({"token_id":id(),"segment_id":s["segment_id"],"text":token.word,"start_utf8":start,"end_utf8":offset}));
                        ranges.push(json!({"segment_id":s["segment_id"],"start_utf8":start,"end_utf8":offset}));
                    }
                }
                Ok(BTreeMap::from([
                    (
                        "tokens".into(),
                        json!({"tokenizer_id":"builtin.tokenize@1.0.0","tokens":tokens}),
                    ),
                    (
                        "map".into(),
                        json!({"coordinate_unit":"utf8_bytes","ranges":ranges}),
                    ),
                ]))
            }
            "builtin.lexical_index" => {
                let batch = input(&inputs, "tokens")?;
                let mut positions: BTreeMap<String, usize> = BTreeMap::new();
                let mut postings: BTreeMap<String, Vec<Value>> = BTreeMap::new();
                for token in items(batch, "tokens")? {
                    let segment_id = text(token, "segment_id")?;
                    let position = positions.entry(segment_id.into()).or_default();
                    postings.entry(text(token, "text")?.to_lowercase()).or_default().push(json!({"token_id":token["token_id"],"segment_id":segment_id,"start_utf8":token["start_utf8"],"end_utf8":token["end_utf8"],"position":*position}));
                    *position += 1;
                }
                Ok(output(
                    "index",
                    json!({"index_id":id(),"tokenizer_id":batch["tokenizer_id"],"tokens":batch["tokens"],"postings":postings.into_iter().map(|(term,occurrences)|json!({"term":term,"occurrences":occurrences})).collect::<Vec<_>>()}),
                ))
            }
            "builtin.search" | "builtin.fuzzy_search" => {
                self.search(inputs, operator == "builtin.fuzzy_search", cancelled)
            }
            "builtin.kwic" => {
                let mut result = Vec::new();
                let texts = items(input(&inputs, "text")?, "segments")?;
                let window = input(&inputs, "window")?;
                let before = window["before"].as_u64().unwrap_or(40).min(500) as usize;
                let after = window["after"].as_u64().unwrap_or(40).min(500) as usize;
                for occurrence in items(input(&inputs, "occurrences")?, "occurrences")? {
                    let range = &occurrence["source"];
                    let segment = texts
                        .iter()
                        .find(|s| s["segment_id"] == range["segment_id"])
                        .ok_or("KWIC missing segment")?;
                    let t = text(segment, "text")?;
                    let start = range["start_utf8"].as_u64().ok_or("missing offset")? as usize;
                    let end = range["end_utf8"].as_u64().ok_or("missing offset")? as usize;
                    let matched = t.get(start..end).ok_or("invalid KWIC range")?;
                    let left: String = t[..start]
                        .chars()
                        .rev()
                        .take(before)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    let right: String = t[end..].chars().take(after).collect();
                    result.push(json!({"occurrence_id":occurrence["occurrence_id"],"segment_id":range["segment_id"],"before":left,"matched":matched,"after":right,"ranges":occurrence["matched_ranges"]}));
                }
                Ok(output("kwic", json!({"items":result})))
            }
            "builtin.translation_candidates" => {
                let contexts = items(input(&inputs, "contexts")?, "contexts")?;
                let alignments = items(input(&inputs, "alignments")?, "contexts")?;
                let mut result = Vec::new();
                for occurrence in items(input(&inputs, "occurrences")?, "occurrences")? {
                    let source = &occurrence["source"];
                    let mut candidates = Vec::new();
                    let mut coverage = "context_missing";
                    for a in alignments {
                        let context = contexts
                            .iter()
                            .find(|c| c["context_id"] == a["context_id"])
                            .ok_or("alignment context missing")?;
                        if !items(context, "sources")?
                            .iter()
                            .any(|s| s["segment_id"] == source["segment_id"])
                        {
                            continue;
                        }
                        let current_coverage = a["coverage"].as_str().unwrap_or("no_links");
                        if current_coverage == "partial" || coverage != "partial" {
                            coverage = current_coverage;
                        }
                        let mut per_segment: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
                        for edge in items(a, "edges")? {
                            let r = &edge["source"];
                            if r["segment_id"] == source["segment_id"]
                                && items(occurrence, "matched_ranges")?.iter().any(|m| {
                                    r["start_utf8"].as_u64() < m["end_utf8"].as_u64()
                                        && r["end_utf8"].as_u64() > m["start_utf8"].as_u64()
                                })
                            {
                                per_segment
                                    .entry(text(&edge["target"], "segment_id")?.into())
                                    .or_default()
                                    .push(edge);
                            }
                        }
                        let mut ranges = Vec::new();
                        let mut content = Vec::new();
                        let mut scores = Vec::new();
                        for (segment_id, mut edges) in per_segment {
                            edges.sort_by_key(|e| e["target"]["start_utf8"].as_u64());
                            let target = items(context, "targets")?
                                .iter()
                                .find(|t| t["segment_id"] == segment_id)
                                .ok_or("target missing")?;
                            let t = text(target, "text")?;
                            let mut merged: Vec<(usize, usize)> = Vec::new();
                            for e in edges {
                                let r = &e["target"];
                                let start =
                                    r["start_utf8"].as_u64().ok_or("offset missing")? as usize;
                                let end = r["end_utf8"].as_u64().ok_or("offset missing")? as usize;
                                if let Some(last) = merged.last_mut()
                                    && start <= last.1
                                {
                                    last.1 = last.1.max(end);
                                } else {
                                    merged.push((start, end));
                                }
                                scores.push(e["score"].as_f64().unwrap_or(0.0));
                            }
                            for (start, end) in merged {
                                content.push(
                                    t.get(start..end).ok_or("invalid model range")?.to_owned(),
                                );
                                ranges.push(json!({"segment_id":segment_id,"start_utf8":start,"end_utf8":end}));
                            }
                        }
                        if !ranges.is_empty() {
                            candidates.push(json!({"ranges":ranges,"text":content.join(" … "),"score":scores.iter().sum::<f64>()/scores.len() as f64,"score_kind":a["score_kind"],"provider_id":a["provider_id"]}));
                        }
                    }
                    result.push(json!({"occurrence_id":occurrence["occurrence_id"],"candidates":candidates,"alignment_coverage":coverage}));
                }
                Ok(output("candidates", json!({"items":result})))
            }
            "builtin.translation_grouping" => {
                let groups: std::collections::BTreeSet<_> =
                    items(input(&inputs, "judgements")?, "items")?
                        .iter()
                        .filter_map(|v| v["group_name"].as_str())
                        .collect();
                let groups: Vec<_> = groups.into_iter().collect();
                let mut suggestions = Vec::new();
                for (i, left) in groups.iter().enumerate() {
                    for right in groups.iter().skip(i + 1) {
                        if left.contains(right) || right.contains(left) {
                            suggestions.push(json!({"left_group":left,"right_group":right,"score":left.chars().count().min(right.chars().count()) as f64/left.chars().count().max(right.chars().count()) as f64,"reason":"表层文字包含关系；须人工判断"}));
                        }
                    }
                }
                Ok(output("suggestions", json!({"items":suggestions})))
            }
            "builtin.export_txt" | "builtin.export_json" | "builtin.export_xml" => {
                let view = input(&inputs, "view")?;
                let side = text(input(&inputs, "spec")?, "side")?;
                let segments: Vec<_> = items(view, "segments")?
                    .iter()
                    .filter(|s| side == "parallel" || s["side"].as_str() == Some(side))
                    .cloned()
                    .collect();
                let mut filtered = view.clone();
                filtered["segments"] = json!(segments);
                if side != "parallel" {
                    filtered["alignments"] = json!([]);
                }
                let (media, content) = match operator {
                    "builtin.export_json" => (
                        "application/json",
                        serde_json::to_string(&filtered).map_err(|e| e.to_string())?,
                    ),
                    "builtin.export_xml" => (
                        "application/xml",
                        format!(
                            "<segments>{}</segments>",
                            segments
                                .iter()
                                .map(|s| format!(
                                    "<segment id=\"{}\">{}</segment>",
                                    xml(s["segment_id"].as_str().unwrap_or("")),
                                    xml(s["text"].as_str().unwrap_or(""))
                                ))
                                .collect::<String>()
                        ),
                    ),
                    _ => (
                        "text/plain",
                        segments
                            .iter()
                            .filter_map(|s| s["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ),
                };
                Ok(output(
                    "export",
                    json!({"media_type":media,"content":content}),
                ))
            }
            _ => self.plugins.execute(operator, inputs, config, cancelled),
        }
    }
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
impl Builtins<'_> {
    fn search(
        &self,
        inputs: Values,
        fuzzy: bool,
        cancelled: &AtomicBool,
    ) -> Result<BTreeMap<String, Value>, String> {
        let query = input(&inputs, "query")?;
        let threshold = if fuzzy {
            query
                .get("minimum_similarity")
                .and_then(Value::as_f64)
                .or_else(|| {
                    default_similarity_threshold(
                        query["similarity_operator"].as_str().unwrap_or(""),
                    )
                })
                .filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0)
                .ok_or("minimum_similarity in (0,1] is required for this provider")?
        } else {
            1.0
        };
        let q = text(query, "text")?.replace('~', " ");
        let terms: Vec<_> = word_ranges(&q)
            .into_iter()
            .map(|(s, e)| q[s..e].to_lowercase())
            .collect();
        if terms.is_empty() || terms.len() > 16 {
            return Err("query requires 1–16 terms".into());
        }
        let gap = query["max_gap"].as_u64().unwrap_or(0).min(10) as usize;
        let mut occurrences = Vec::new();
        let source = input(&inputs, "segments")?;
        let indexed = source.get("postings").is_some();
        let documents = items(source, if indexed { "items" } else { "segments" })?;
        let eligible = if indexed && !fuzzy {
            let mut intersection: Option<BTreeSet<&str>> = None;
            for term in &terms {
                let ids: BTreeSet<_> = items(source, "postings")?
                    .iter()
                    .find(|p| p["term"].as_str() == Some(term.as_str()))
                    .and_then(|p| p["segment_ids"].as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                intersection = Some(match intersection {
                    Some(previous) => previous.intersection(&ids).copied().collect(),
                    None => ids,
                });
            }
            intersection
        } else {
            None
        };
        for segment in documents {
            if eligible
                .as_ref()
                .is_some_and(|ids| !ids.contains(segment["segment_id"].as_str().unwrap_or("")))
            {
                continue;
            }
            if cancelled.load(Ordering::Acquire) {
                return Err("cancelled".into());
            }
            let content = text(segment, "text")?;
            let words = word_ranges(content);
            let mut scores = vec![vec![0.0; words.len()]; terms.len()];
            for (qi, term) in terms.iter().enumerate() {
                if fuzzy {
                    for (chunk_index, chunk) in words.chunks(1024).enumerate() {
                        let pairs:Vec<_>=chunk.iter().enumerate().map(|(i,(s,e))|json!({"pair_id":(chunk_index*1024+i).to_string(),"left":term,"right":&content[*s..*e]})).collect();
                        let value = self.plugins.execute(
                            text(query, "similarity_operator")?,
                            BTreeMap::from([("pairs".into(), vec![json!({"pairs":pairs})])]),
                            &json!({}),
                            cancelled,
                        )?;
                        for item in items(value.get("scores").ok_or("missing scores")?, "items")? {
                            let index: usize = text(item, "pair_id")?
                                .parse()
                                .map_err(|_| "invalid score index")?;
                            if index >= words.len() {
                                return Err("score index out of range".into());
                            }
                            scores[qi][index] = item["score"].as_f64().ok_or("missing score")?;
                        }
                    }
                } else {
                    for (i, (s, e)) in words.iter().enumerate() {
                        scores[qi][i] = if content[*s..*e].to_lowercase() == *term {
                            1.0
                        } else {
                            0.0
                        };
                    }
                }
            }
            for start in 0..words.len() {
                if scores[0][start] < threshold {
                    continue;
                }
                let mut matched = vec![start];
                let mut previous = start;
                let mut remaining = gap;
                for row in scores.iter().skip(1) {
                    let end = (previous + remaining + 2).min(words.len());
                    if let Some(next) = (previous + 1..end).find(|i| row[*i] >= threshold) {
                        remaining -= next - previous - 1;
                        matched.push(next);
                        previous = next;
                    } else {
                        break;
                    }
                }
                if matched.len() != terms.len() {
                    continue;
                }
                let ranges:Vec<_>=matched.iter().map(|i|json!({"segment_id":segment["segment_id"],"start_utf8":words[*i].0,"end_utf8":words[*i].1})).collect();
                let (s, e) = (words[start].0, words[previous].1);
                occurrences.push(json!({"occurrence_id":id(),"source":{"segment_id":segment["segment_id"],"start_utf8":s,"end_utf8":e},"matched_ranges":ranges,"text":&content[s..e]}));
                if occurrences.len() > 10000 {
                    return Err("batch occurrence limit exceeded; narrow query".into());
                }
            }
        }
        Ok(output("occurrences", json!({"occurrences":occurrences})))
    }
}
