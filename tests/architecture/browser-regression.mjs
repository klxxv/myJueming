// Run against pnpm dev:web; Playwright and Chromium must be available.
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
import assert from 'node:assert/strict';
const browser = await chromium.launch({headless:true});
const page = await browser.newPage({locale:'zh-CN',viewport:{width:1440,height:1000}});
const errors=[]; page.on('pageerror', e=>errors.push(e.message));
await page.addInitScript(() => {
  localStorage.setItem('jueming-parallel-tutorial-launched-v2', '1');
  const calls = []; const texts = {}; const callbacks = new Map(); let callbackId=0;
  const projectId = crypto.randomUUID(), sourceDoc=crypto.randomUUID(), targetDoc=crypto.randomUUID();
  const view = {contract_version:'1.0', project:{project_id:projectId,name:'Slice browser verification',source_language:'zh',target_language:'en',document_ids:[sourceDoc,targetDoc],current_revision_id:'1',format_version:'1.0',created_at:'2026-09-13T00:00:00Z',updated_at:'2026-09-13T00:00:00Z'},documents:[{document_id:sourceDoc,project_id:projectId,title:'source'},{document_id:targetDoc,project_id:projectId,title:'target'}], segments:[],segment_orders:[{document_id:sourceDoc,entries:[]},{document_id:targetDoc,entries:[]}],alignments:[],revisions:[{revision_id:'1',project_id:projectId,created_at:'2026-09-13T00:00:00Z',change_set:{operation:'create_project',affected_count:1600},summary:'created',state:'complete'}], bookmarks:[],annotations:[],summary:{project_id:projectId,name:'Slice browser verification',source_label:'source.txt',target_label:'target.txt',source_count:800,target_count:800,alignment_count:800,source_unlinked_count:0,target_unlinked_count:0,revision_id:'1'}};
  for(let i=0;i<800;i++) {
    const source=crypto.randomUUID(),target=crypto.randomUUID();
    for(const [id,doc,text,side] of [[source,sourceDoc,`中文正文 第${i}句`,0],[target,targetDoc,`English sentence ${i}`,1]]) {texts[id]=text;view.segments.push({segment_id:id,document_id:doc,content_hash:id,content_length:text.length});view.segment_orders[side].entries.push({segment_id:id,position_key:String(i)});}
    view.alignments.push({alignment_id:crypto.randomUUID(),source_segment_ids:[source],target_segment_ids:[target],producer:'manual',cardinality:'1:1'});
  }
  window.audit={calls,view,texts,failSave:false};
  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
  window.__TAURI_INTERNALS__={ metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(fn){callbacks.set(++callbackId,fn);return callbackId;}, unregisterCallback(id){callbacks.delete(id);},invoke:async(cmd,args={})=>{
    calls.push({cmd,args});
    if(cmd==='load_app_settings') return null;
    if(cmd==='get_current_project') return structuredClone(view);
    if(cmd==='get_project_summary')return structuredClone(view.summary);
    if(cmd==='list_bookmarks')return [];
    if(cmd==='agent_projection')return {request_id:'projection',sequence:'0',data:{project:{project:view.project},binding_id:null,context:null,search_spec:{query:'',regex:false,case_sensitive:false,language_id:null},search_results:null,proposals:[],pending_ui_actions:[]}};
    if(cmd==='agent_call')return {request_id:args.call.request_id,sequence:'0',data:args.call.method==='app.bind_session'?{binding_id:'binding',project_id:projectId,revision_id:view.project.current_revision_id}:args.call.method==='pipeline.list_proposals'?[]:{}};
    if(cmd==='agent_runtime_status')return {configured:false,available:false,active_run_ids:[],recovered_interrupted_runs:0};
    if(cmd==='agent_runtime_history')return {session_id:'session',messages:[]};
    if(cmd.startsWith('plugin:event|'))return 1;
    if(cmd==='load_parallel_slice') {if(args.request.revision_id!==view.project.current_revision_id)throw {code:'stale_revision',message:'stale'};return {project_id:projectId,revision_id:view.project.current_revision_id,segments:args.request.segment_ids.map(id=>({segment_id:id,content:texts[id],content_hash:view.segments.find(s=>s.segment_id===id).content_hash}))};}
    if(cmd==='execute_command') {
      if(window.audit.failSave)throw {code:'io_error',message:'simulated disk failure'};
      const command=args.command;
      if(command.base_revision_id!==view.project.current_revision_id)throw {code:'stale_revision',message:'stale'};
      if(command.kind==='update_segment') {texts[command.payload.segment_id]=command.payload.content;const segment=view.segments.find(s=>s.segment_id===command.payload.segment_id);segment.content_hash=crypto.randomUUID();segment.content_length=command.payload.content.length;}
      const revision=String(BigInt(view.project.current_revision_id)+1n);view.project.current_revision_id=revision;view.summary.revision_id=revision;view.revisions.push({revision_id:revision,created_at:'2026-09-13T00:00:00Z',change_set:{operation:command.kind,affected_count:1},summary:'changed',state:'complete'});
      return {project_id:projectId,command_id:command.command_id,committed_revision_id:revision,status:'committed'};
    }
    if(cmd==='plugin:dialog|open')return '/tmp/another.jm';
    if(cmd==='open_project')return structuredClone(view);
    return null;
  }};
});
try {
  await page.goto('http://127.0.0.1:1420');
  await page.getByText('中文正文 第0句',{exact:true}).waitFor();
  const initial=await page.evaluate(()=>({slices:window.audit.calls.filter(c=>c.cmd==='load_parallel_slice').map(c=>c.args.request.segment_ids.length),dom:document.querySelectorAll('[data-segment-content]').length}));
  assert.ok(initial.dom<100); assert.ok(initial.slices.reduce((a,b)=>a+b,0)<200);
  await page.locator('.aligned-workspace-viewport').evaluate(el=>{el.scrollTop=el.scrollHeight;});
  await page.getByText('中文正文 第799句',{exact:true}).waitFor();
  await page.getByText('中文正文 第799句',{exact:true}).dblclick();
  const textarea=page.locator('.segment-card__editor textarea');await textarea.fill('保留失败的编辑草稿');
  await page.evaluate(()=>{window.audit.failSave=true;});
  await page.getByRole('button',{name:'打开',exact:true}).click();
  const guard=page.getByRole('dialog',{name:'当前句段还有未保存编辑'});await guard.waitFor();
  await guard.getByRole('button',{name:'保存并切换'}).click();
  await page.getByText('simulated disk failure',{exact:false}).first().waitFor();
  assert.equal(await page.evaluate(()=>window.audit.calls.filter(c=>c.cmd==='open_project').length),0);
  await guard.getByRole('button',{name:'继续编辑'}).click();
  assert.equal(await textarea.inputValue(),'保留失败的编辑草稿');
  await page.getByRole('button',{name:'打开',exact:true}).click(); await guard.waitFor();
  await page.evaluate(()=>{window.audit.failSave=false;});
  await guard.getByRole('button',{name:'保存并切换'}).click();
  await guard.waitFor({state:'hidden'});
  const saves=await page.evaluate(()=>window.audit.calls.filter(c=>c.cmd==='execute_command').map(c=>c.args.command));
  assert.equal(saves[0].command_id,saves[1].command_id);
  assert.equal(saves[0].base_revision_id,'1');
  await page.locator('select[aria-label="切换工作模式"]').selectOption('order');
  await page.locator('select[aria-label="切换工作模式"]').selectOption('review');
  await page.keyboard.press('Meta+f');
  await page.getByRole('textbox',{name:'查找当前平行视图'}).fill('English sentence 723');
  await page.getByText('1 / 1',{exact:true}).waitFor();
  await page.getByRole('button',{name:'关闭查找'}).click();
  await page.getByRole('button',{name:'批注',exact:true}).click();
  await page.getByRole('button',{name:'新建批注',exact:true}).click();
  const annotationForm=page.locator('.annotation-editor--new');
  await annotationForm.getByLabel('标题',{exact:true}).fill('保留批注草稿');
  await annotationForm.getByLabel('内容',{exact:true}).fill('正文不能丢失');
  await page.evaluate(()=>{window.audit.failSave=true;});
  await annotationForm.getByRole('button',{name:'创建',exact:true}).click();
  await page.waitForFunction(()=>window.audit.calls.filter(c=>c.cmd==='execute_command'&&c.args.command.kind==='create_annotation').length===1);
  assert.equal(await annotationForm.getByLabel('内容',{exact:true}).inputValue(),'正文不能丢失');
  await page.evaluate(()=>{window.audit.failSave=false;});
  await annotationForm.getByRole('button',{name:'创建',exact:true}).click();
  await annotationForm.waitFor({state:'hidden'});
  const annotationSaves=await page.evaluate(()=>window.audit.calls.filter(c=>c.cmd==='execute_command'&&c.args.command.kind==='create_annotation').map(c=>c.args.command));
  assert.equal(annotationSaves[0].command_id,annotationSaves[1].command_id);
  assert.equal(annotationSaves[0].base_revision_id,'2');
  if (process.env.ARCHITECTURE_SCREENSHOT) await page.screenshot({path:process.env.ARCHITECTURE_SCREENSHOT});
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({initial,saves:saves.length,result:'actual App: lazy body loading, far scrolling, edit guard failure/cancel/retry, mode switching, View Find, and annotation failure/retry passed'}));
} finally {await browser.close();}
