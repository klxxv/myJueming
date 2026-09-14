"""Run the actual managed XLM-R worker against a tiny labelled engineering fixture."""
import argparse,json,platform,subprocess,time,uuid,os
try:
    import resource
except ImportError:
    resource = None
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--bundle',required=True,type=Path);p.add_argument('--output',required=True,type=Path);args=p.parse_args()
root=Path(__file__).resolve().parents[2];architecture='aarch64' if platform.machine().lower() in ('arm64','aarch64') else 'x86_64'
manifest_path=args.bundle/f'bundle.{architecture}.json'
if not manifest_path.exists():manifest_path=args.bundle/'bundle.json'
manifest=json.loads(manifest_path.read_text())
fixture=json.loads((root/'tests/fixtures/research/xlmr-alignment-v1.json').read_text())
env={**os.environ,'PYTHONPATH':str(args.bundle/manifest['packages']),'PYTHONNOUSERSITE':'1','PYTHONDONTWRITEBYTECODE':'1','HF_HUB_OFFLINE':'1','TRANSFORMERS_OFFLINE':'1','JUEMING_MODEL_DIR':str(args.bundle/manifest['model']),'TOKENIZERS_PARALLELISM':'false'};env.pop('PYTHONHOME',None)
worker=subprocess.Popen([str(args.bundle/manifest['python']),'-B','-s','-u',str(args.bundle/'plugins/xlmr-word-alignment/worker.py')],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
def call(payload):
    identity=str(uuid.uuid4());worker.stdin.write(json.dumps({'request_id':identity,**payload})+'\n');worker.stdin.flush();result=json.loads(worker.stdout.readline());assert result['request_id']==identity,result
    if 'error' in result:raise RuntimeError(result['error'])
    return result['data']
def span(text,part):
    start=text.index(part);return len(text[:start].encode()),len(text[:start+len(part)].encode())
results=[];start=time.perf_counter()
try:
    call({'method':'hello'});load_seconds=time.perf_counter()-start
    for case in fixture['cases']:
        source={'segment_id':str(uuid.uuid4()),'text':case['source'],'language':'en'}
        targets=[{'segment_id':str(uuid.uuid4()),'text':text,'language':'zh-CN'} for text in case['targets']];cid=str(uuid.uuid4());qs,qe=span(case['source'],case['query']);started=time.perf_counter()
        aligned=call({'method':'execute','operator_id':'xlmr.contextual_alignment','config':{},'inputs':{'contexts':[{'contexts':[{'context_id':cid,'sources':[source],'targets':targets}]}]}})['alignments']['contexts'][0]
        predicted=set();gold=set()
        for edge in aligned['edges']:
            if edge['source']['start_utf8']<qe and edge['source']['end_utf8']>qs:
                t=edge['target'];predicted.update((t['segment_id'],i) for i in range(t['start_utf8'],t['end_utf8']))
        for expected in case['gold']:
            target=targets[expected['target']];a,b=span(target['text'],expected['text']);gold.update((target['segment_id'],i) for i in range(a,b))
        fragments=[]
        for target in targets:
            data=target['text'].encode();positions=[i for i in range(len(data)) if (target['segment_id'],i) in predicted];fragments.append(bytes(data[i] for i in positions).decode())
        hit=len(predicted&gold)
        results.append({'id':case['id'],'predicted_text':fragments,'gold':[g['text'] for g in case['gold']],'coverage':aligned['coverage'],'exact':predicted==gold,'byte_precision':hit/len(predicted) if predicted else None,'byte_recall':hit/len(gold) if gold else None,'seconds':round(time.perf_counter()-started,3)})
finally:
    worker.kill();worker.wait()
report={'fixture_version':fixture['version'],'caveat':fixture['purpose'],'platform':platform.platform(),'cpu':platform.processor(),'model_revision':manifest['model_revision'],'method':'layer8 contextual cosine mutual-nearest >= 0.15','model_load_seconds':round(load_seconds,3),'exact_cases':sum(r['exact'] for r in results),'cases':len(results),'worker_peak_rss_native_units':resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss if resource else None,'rss_unit':'bytes on macOS; KiB on Linux','results':results}
args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(json.dumps(report,ensure_ascii=False,indent=2))
