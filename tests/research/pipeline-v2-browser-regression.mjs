// Run against pnpm dev:web. This exercises the real Vue application with a test Host bridge; it does not validate algorithm output.
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
import assert from 'node:assert/strict';
const browser = await chromium.launch({headless:true});
const page = await browser.newPage({viewport:{width:1440,height:1000}});
const errors=[]; page.on('pageerror', e=>errors.push(e.message));
await page.addInitScript(() => {
  const calls = []; const texts = {}; const callbacks = new Map(); let callbackId=0;
  const projectId = crypto.randomUUID(), sourceDoc=crypto.randomUUID(), targetDoc=crypto.randomUUID();
  const view = {contract_version:'1.0', project:{project_id:projectId,name:'Research browser verification',source_language:'zh',target_language:'en',document_ids:[sourceDoc,targetDoc],current_revision_id:'1',format_version:'1.0',created_at:'2026-09-13T00:00:00Z',updated_at:'2026-09-13T00:00:00Z'},documents:[{document_id:sourceDoc,project_id:projectId,title:'source'},{document_id:targetDoc,project_id:projectId,title:'target'}], segments:[],segment_orders:[{document_id:sourceDoc,entries:[]},{document_id:targetDoc,entries:[]}],alignments:[],revisions:[{revision_id:'1',project_id:projectId,created_at:'2026-09-13T00:00:00Z',change_set:{operation:'create_project',affected_count:1600},summary:'created',state:'complete'}], bookmarks:[],annotations:[],summary:{project_id:projectId,name:'Research browser verification',source_label:'source.txt',target_label:'target.txt',source_count:800,target_count:800,alignment_count:800,source_unlinked_count:0,target_unlinked_count:0,revision_id:'1'}};
  for(let i=0;i<800;i++) {
    const source=crypto.randomUUID(),target=crypto.randomUUID();
    for(const [id,doc,text,side] of [[source,sourceDoc,`中文正文 第${i}句`,0],[target,targetDoc,`English sentence ${i}`,1]]) {texts[id]=text;view.segments.push({segment_id:id,document_id:doc,content_hash:id,content_length:text.length});view.segment_orders[side].entries.push({segment_id:id,position_key:String(i)});}
    view.alignments.push({alignment_id:crypto.randomUUID(),source_segment_ids:[source],target_segment_ids:[target],producer:'manual',cardinality:'1:1'});
  }
  window.audit={calls,view,texts,failSave:false,runs:[],confirmations:[]};
  const feature={feature_id:'translation_research',desired_enabled:false,status:'disabled',stage:null,generation:'0',activation_id:null,completed_bytes:null,total_bytes:null,reason:null,resources_ready:true,worker_state:'stopped',default_similarity:'fuzzy.edit_distance',auto_locate:true};
  if (location.search.includes('research_history_fixture=1')) window.audit.runs=[{run_id:'restored-run',project_id:projectId,input_revision_id:'1',status:'completed',error:null,total:250,completed:250}];
  Object.assign(feature,{desired_enabled:true,status:'ready'});
  view.project.current_revision_id='2';view.summary.revision_id='2';view.revisions.push({...view.revisions[0],revision_id:'2',summary:'prior edit'});
  window.audit.methods=[];window.audit.graphRunStatus='running';window.audit.invalidPlan=false;
  const textSchema={name:'TextView',version:1};
  const inputPort={name:'value',schemas:[textSchema],required:true,multiple:false};
  const transformInput={name:'text',schemas:[textSchema],required:true,multiple:false};
  const transformOutput={name:'result',schemas:[textSchema],required:true,multiple:false};
  const querySchema={name:'QuerySpec',version:1};
  const operators=[{operator_id:'host.input.QuerySpec',name:'静态查询输入',release:'1',slots:['runtime.input.QuerySpec'],inputs:[],outputs:[{...inputPort,schemas:[querySchema]}],config_schema:{type:'object'}},{operator_id:'host.input.TextView',name:'工程文本输入',release:'1',slots:['runtime.input.TextView'],inputs:[],outputs:[inputPort],config_schema:{type:'object'}},...['a','b'].map(suffix=>({operator_id:'test.transform.'+suffix,name:'Transform '+suffix.toUpperCase(),release:'1',slots:['transform.text'],inputs:[transformInput],outputs:[transformOutput],config_schema:{type:'object'}}))];
  const slots=[{slot_id:'runtime.input.TextView',name:'工程输入',state:'bound',reason:null,inputs:[],outputs:[inputPort],providers:['host.input.TextView']},{slot_id:'transform.text',name:'文本处理',state:'bound',reason:null,inputs:[transformInput],outputs:[transformOutput],providers:['test.transform.a','test.transform.b']},{slot_id:'token.pos',name:'词性',state:'unbound',reason:'没有可用的算法实现',inputs:[],outputs:[transformOutput],providers:[]}];
  const occurrence=(index)=>({occurrence_id:'occurrence-'+index,source:{segment_id:view.segments[0].segment_id,text:'heavy rain and heavy rain',ranges:[{start_utf8:0,end_utf8:10}]},targets:[{segment_id:view.segments[1].segment_id,text:'今天有大雨。'}],candidates:[{ranges:[{segment_id:view.segments[1].segment_id,start_utf8:9,end_utf8:15}],text:'大雨',score:.85,score_kind:'alignment_similarity',provider_id:'xlmr.word_alignment'}],status:'pending',judgement:null});

  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
  window.__TAURI_INTERNALS__={ metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(fn){callbacks.set(++callbackId,fn);return callbackId;}, unregisterCallback(id){callbacks.delete(id);},invoke:async(cmd,args={})=>{
    calls.push({cmd,args});
    if(cmd==='load_app_settings') return null;
    if(cmd==='get_current_project') return structuredClone(view);
    if(cmd==='get_project_summary')return structuredClone(view.summary);
    if(cmd==='list_bookmarks')return [];
    if(cmd==='agent_projection')return {request_id:'projection',sequence:'0',data:{project:{project:view.project},binding_id:null,context:null,search_spec:{query:'',regex:false,case_sensitive:false,language_id:null},search_results:null,proposals:[],pending_ui_actions:[]}};
    if(cmd==='agent_call') {
      const {method,params}=args.call; let data={};
      if(method==='app.bind_session')data={binding_id:'binding',project_id:projectId,revision_id:view.project.current_revision_id};
      else if(method==='pipeline.list_proposals'||method==='pipeline.list')data=[];
      else if(method==='operators.list')data=structuredClone(operators);
      else if(method==='slots.list')data=structuredClone(slots);
      else if(method==='schemas.get')data={...params,schema:{type:'object',properties:{segments:{type:'array'}},required:['segments']}};
      else if(method==='pipeline.list_methods_v2')data=structuredClone(window.audit.methods);
      else if(method==='pipeline.validate_plan'){
        if(window.audit.invalidPlan)throw {code:'invalid_plan',message:'Host rejected incompatible port'};
        if(params.format_version!==2||params.nodes.length!==2||!params.nodes[1].inputs.text?.length)throw {code:'invalid_plan',message:'Named input is required'};
        if(!params.outputs.some(output=>output.port==='result'))throw {code:'invalid_plan',message:'Select result output'};
        data=params.nodes.map(node=>node.node_id);
      }
      else if(method==='pipeline.save_method_v2'){data={method_id:params.method_id??crypto.randomUUID(),name:params.name,method_revision_id:crypto.randomUUID(),plan:structuredClone(params.plan)};window.audit.methods=[data];}
      else if(method==='pipeline.start'){const dataRun={run_id:crypto.randomUUID(),project_id:projectId,input_revision_id:'1',status:window.audit.graphRunStatus,error:null,total:2,completed:window.audit.graphRunStatus==='completed'?2:0};window.audit.graphRun=dataRun;data=structuredClone(dataRun);}
      else if(method==='pipeline.get_graph_run')data={...window.audit.graphRun,status:window.audit.graphRunStatus,artifacts:window.audit.graphRunStatus==='completed'?[{handle:'artifact-handle',project_id:projectId,input_revision_id:'1',run_id:params.run_id,schema:textSchema,sha256:'abc',bytes:42,provider_id:'test.transform.b',provider_release:'1',dependencies:[]}]:[]};
      else if(method==='pipeline.cancel_graph_run'){window.audit.graphRunStatus='cancelled';data={...window.audit.graphRun,status:'cancelled'};}
      else if(method==='capabilities.get')data={contract_version:'2',generation:feature.generation,features:[structuredClone(feature)],slots:[],operators:[]};
      else if(method==='features.enable'||method==='features.retry_prepare'){Object.assign(feature,{desired_enabled:true,status:'ready',worker_state:'idle',generation:String(+feature.generation+1)});data=structuredClone(feature);}
      else if(method==='features.disable'||method==='features.cancel_prepare'){Object.assign(feature,{desired_enabled:false,status:'disabled',worker_state:'stopped',generation:String(+feature.generation+1)});data=structuredClone(feature);}
      else if(method==='features.update_preferences'){Object.assign(feature,params);data=structuredClone(feature);}
      else if(method==='pipeline.list_runs')data=structuredClone(window.audit.runs);
      else if(method==='research.start'){data={run_id:'run-'+(window.audit.runs.length+1),project_id:projectId,input_revision_id:'1',status:'completed',error:null,total:250,completed:250};window.audit.runs.unshift(data);}
      else if(method==='pipeline.get_run')data=structuredClone(window.audit.runs.find(run=>run.run_id===params.run_id));
      else if(method==='pipeline.read_result')data={run:structuredClone(window.audit.runs.find(run=>run.run_id===params.run_id)),items:Array.from({length:Math.min(params.limit,250-params.cursor)},(_,i)=>occurrence(params.cursor+i)),total:250,next_cursor:params.cursor+params.limit<250?params.cursor+params.limit:null};
      else if(method==='research.confirm'){if(window.audit.failSave)throw {code:'io_error',message:'research disk failure'};window.audit.confirmations.push(params);data={committed_revision_id:'2'};}
      return {request_id:args.call.request_id,sequence:'0',data};
    }
    if(cmd==='agent_runtime_status')return {configured:false,available:false,active_run_ids:[],recovered_interrupted_runs:0};
    if(cmd==='agent_runtime_history')return {session_id:'session',messages:[]};
    if(cmd.startsWith('plugin:event|'))return 1;
    if(cmd==='load_parallel_slice') {if(args.request.revision_id!==view.project.current_revision_id)throw {code:'stale_revision',message:'stale'};return {project_id:projectId,revision_id:view.project.current_revision_id,segments:args.request.segment_ids.map(id=>({segment_id:id,content:texts[id],content_hash:view.segments.find(s=>s.segment_id===id).content_hash}))};}
    if(cmd==='execute_command') {
      if(window.audit.failSave)throw {code:'io_error',message:'simulated disk failure'};
      const command=args.command;
      if(command.base_revision_id!==view.project.current_revision_id)throw {code:'stale_revision',message:'stale'};
      if(command.kind==='update_segment') {texts[command.payload.segment_id]=command.payload.content;const segment=view.segments.find(s=>s.segment_id===command.payload.segment_id);segment.content_hash=crypto.randomUUID();segment.content_length=command.payload.content.length;}
      const revision=String(BigInt(view.project.current_revision_id)+1n);view.project.current_revision_id=revision;view.summary.revision_id=revision;view.revisions.push({revision_id:revision,created_at:'2026-09-13T00:00:00Z',change_set:{operation:command.kind==='undo'?'undo:test':command.kind,affected_count:1},summary:'changed',state:'complete'});
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
  await page.locator('[data-nav-id="pipeline"]').click();
  await page.getByRole('button',{name:'新建数据流',exact:true}).click();
  await page.getByRole('textbox',{name:'数据流方法名称'}).fill('具名端口测试');
  await page.getByLabel('添加算子',{exact:true}).selectOption('host.input.QuerySpec');
  await page.getByRole('button',{name:'添加节点',exact:true}).click();
  assert.deepEqual(JSON.parse(await page.getByLabel('节点 1 参数 JSON',{exact:true}).inputValue()),{value:{}},'Non-text schema never receives a view adapter');
  await page.getByRole('button',{name:'删除节点 1',exact:true}).click();
  await page.getByLabel('添加算子',{exact:true}).selectOption('host.input.TextView');
  await page.getByRole('button',{name:'添加节点',exact:true}).click();
  assert.deepEqual(JSON.parse(await page.getByLabel('节点 1 参数 JSON',{exact:true}).inputValue()),{view:'source'});
  await page.getByLabel('添加算子',{exact:true}).selectOption('test.transform.a');
  await page.getByRole('button',{name:'添加节点',exact:true}).click();
  const nodes=page.locator('.v2-node');
  const inputId=await nodes.first().getAttribute('data-pipeline-node-id');
  await page.getByLabel('节点 2 输入 text 1',{exact:true}).selectOption(JSON.stringify([inputId,'value']));
  await page.getByLabel('节点 2 参数 JSON',{exact:true}).fill('{"case":"upper"}');
  const outputs=page.getByRole('group',{name:'发布为方法结果的输出'}).getByRole('checkbox');
  await outputs.first().uncheck();await outputs.last().check();
  await nodes.first().getByRole('button',{name:'查看合同',exact:true}).click();
  await page.locator('.v2-schema pre').waitFor();
  await page.evaluate(()=>window.audit.invalidPlan=true);
  await page.getByRole('button',{name:'校验连接',exact:true}).click();
  await page.getByText('Host rejected incompatible port',{exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>window.audit.methods.length),0);
  await page.evaluate(()=>window.audit.invalidPlan=false);
  await page.getByRole('button',{name:'校验连接',exact:true}).click();
  await page.getByText('Host 校验通过：',{exact:false}).waitFor();
  await page.getByRole('button',{name:'保存方法',exact:true}).click();
  await page.waitForFunction(()=>window.audit.methods.length===1);
  const saved=await page.evaluate(()=>window.audit.methods[0]);
  assert.equal(saved.plan.nodes[1].inputs.text[0].node_id,inputId);
  assert.equal(saved.plan.nodes[1].inputs.text[0].port,'value');
  assert.equal(saved.plan.nodes[1].config.case,'upper');
  assert.match(inputId,/^[0-9a-f-]{36}$/);
  await page.getByRole('textbox',{name:'数据流方法名称'}).fill('修改后的方法');
  await page.locator('[data-nav-id="parallel"]').click();
  const guard=page.getByRole('dialog',{name:'处理流程还有未保存草稿'});await guard.waitFor();
  await guard.getByRole('button',{name:'继续编辑',exact:true}).click();
  const historyCount=()=>page.evaluate(()=>window.audit.calls.filter(call=>call.cmd==='execute_command'&&['undo','redo'].includes(call.args.command.kind)).length);
  await page.locator('.app-toolbar').getByRole('button',{name:'撤销',exact:true}).click();
  await guard.waitFor();assert.equal(await historyCount(),0);
  await guard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.equal(await page.getByRole('textbox',{name:'数据流方法名称'}).inputValue(),'修改后的方法');
  await page.locator('.app-toolbar').getByRole('button',{name:'撤销',exact:true}).click();
  await guard.getByRole('button',{name:'保存并继续',exact:true}).click();
  await page.getByText('已撤销并保存为新的 Revision',{exact:false}).waitFor();
  const savedBeforeUndo=await page.evaluate(()=>window.audit.methods[0]);
  assert.equal(savedBeforeUndo.name,'修改后的方法');
  assert.equal(await historyCount(),1);
  await page.getByRole('textbox',{name:'数据流方法名称'}).fill('重做前的处理流程草稿');
  await page.locator('.app-toolbar').getByRole('button',{name:'重做',exact:true}).click();
  await guard.waitFor();await guard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.equal(await historyCount(),1);
  assert.equal(await page.getByRole('textbox',{name:'数据流方法名称'}).inputValue(),'重做前的处理流程草稿');
  await page.locator('.app-toolbar').getByRole('button',{name:'重做',exact:true}).click();
  await guard.getByRole('button',{name:'放弃草稿',exact:true}).click();
  await page.getByText('已重做并保存为新的 Revision',{exact:false}).waitFor();
  assert.equal(await page.getByRole('textbox',{name:'数据流方法名称'}).inputValue(),'修改后的方法');
  await page.getByLabel('节点 2 算法实现',{exact:true}).selectOption('test.transform.b');
  assert.equal(await page.getByLabel('节点 2 输入 text 1',{exact:true}).inputValue(),JSON.stringify([inputId,'value']));
  await page.getByRole('button',{name:'保存方法',exact:true}).click();
  await page.waitForFunction(old=>window.audit.methods[0].method_revision_id!==old,savedBeforeUndo.method_revision_id);
  const saveCall=await page.evaluate(()=>window.audit.calls.filter(call=>call.args?.call?.method==='pipeline.save_method_v2').at(-1).args.call.params);
  assert.equal(saveCall.base_method_revision_id,savedBeforeUndo.method_revision_id);
  assert.equal(saveCall.plan.nodes[1].operator_id,'test.transform.b');
  await page.getByRole('button',{name:'运行已保存版本',exact:true}).click();
  await page.getByRole('button',{name:'停止',exact:true}).click();
  await page.getByText('已停止',{exact:true}).waitFor();
  await page.evaluate(()=>window.audit.graphRunStatus='completed');
  await page.getByRole('button',{name:'运行已保存版本',exact:true}).click();
  await page.getByText('artifact-handle',{exact:true}).waitFor();
  await page.getByText('运行完成',{exact:true}).waitFor();
  await page.getByRole('button',{name:'分词方法 v1',exact:true}).click();
  await page.getByRole('button',{name:'创建中文分词方法',exact:true}).waitFor();
  await page.getByRole('button',{name:'数据流 v2',exact:true}).click();
  if(process.env.PIPELINE_V2_SCREENSHOT)await page.screenshot({path:process.env.PIPELINE_V2_SCREENSHOT});
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({result:'actual Vue v2 editor passed',checks:['Registry-driven operator/Slot/Schema UI','opaque node IDs and named port wiring','Host validation rejects invalid plans','versioned save','method draft guard','provider switch retains compatible ports','run cancellation','immutable artifact references','v1 coexistence','non-text input schemas use value instead of view','Undo saves Pipeline draft first and cancellation retains edits','Redo guards Pipeline draft']}));
} catch(error) {console.error(await page.locator('body').innerText());console.error(await page.evaluate(()=>window.audit.calls.slice(-12)));throw error;} finally {await browser.close();}
