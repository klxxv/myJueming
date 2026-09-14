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
  window.audit={calls,view,texts,failSave:false,runs:[],confirmations:[],merges:[],listeners:{},sequence:0,restoreApproved:true,commitConfirmRevision:false,staleJudgements:false};
  window.audit.advanceRevision=(operation='undo merge',emit=true)=>{
    const revision=String(BigInt(view.project.current_revision_id)+1n);
    view.project.current_revision_id=revision;view.summary.revision_id=revision;
    view.revisions.push({revision_id:revision,project_id:projectId,created_at:'2026-09-13T00:00:00Z',change_set:{operation,affected_count:1},summary:operation,state:'complete'});
    if(!emit)return;
    const event={contract_version:'1.0',sequence:String(++window.audit.sequence),kind:'revision_advanced',binding_id:'binding',origin:'native',payload:{project_id:projectId,revision_id:revision}};
    for(const handler of window.audit.listeners.agent_subscribe??[])callbacks.get(handler)?.({event:'agent_subscribe',id:1,payload:event});
  };
  window.audit.emitRevision=()=>window.audit.advanceRevision();
  const feature={feature_id:'translation_research',desired_enabled:false,status:'disabled',stage:null,generation:'0',activation_id:null,completed_bytes:null,total_bytes:null,reason:null,resources_ready:true,worker_state:'stopped',default_similarity:'fuzzy.edit_distance',auto_locate:true};
  if (location.search.includes('research_history_fixture=1')) window.audit.runs=[{run_id:'restored-run',project_id:projectId,input_revision_id:'1',status:'completed',error:null,total:250,completed:250}];
  const mergedName=(name,runId)=>window.audit.merges.filter(merge=>merge.run_id===runId).reduce((value,merge)=>value===merge.from_group?merge.to_group:value,name);
  const occurrence=(index,runId)=>{
    const range={segment_id:view.segments[1].segment_id,start_utf8:9,end_utf8:15};
    const coverage=['partial','no_links','not_requested','context_missing','unknown','complete',null][index];
    const confirmed=window.audit.confirmations.filter(item=>item.run_id===runId&&item.occurrence_id==='occurrence-'+index).at(-1);
    const judgement=confirmed?{kind:confirmed.kind,group_name:mergedName(confirmed.group_name,runId),strategy:confirmed.strategy??'',target_ranges:confirmed.target_ranges}:index>=200&&index<=210?{kind:'translation',group_name:mergedName(index<208?'大雨':'大雨点',runId),strategy:index<208?'直译':'强化',target_ranges:[range]}:null;
    return {occurrence_id:'occurrence-'+index,source:{segment_id:view.segments[0].segment_id,text:'heavy rain and heavy rain',ranges:[{start_utf8:0,end_utf8:10}]},targets:[{segment_id:view.segments[1].segment_id,text:'今天有大雨。'}],candidates:['no_links','not_requested','context_missing'].includes(coverage)?[]:[{ranges:[range],text:'大雨',score:.85,score_kind:'alignment_similarity',provider_id:'xlmr.word_alignment'}],alignment_coverage:coverage,status:judgement?(window.audit.staleJudgements&&index===0?'needs_review':'confirmed'):'pending',judgement};
  };
  const summaryFor=(runId)=>{
    const changed=[...new Map(window.audit.confirmations.filter(item=>item.run_id===runId).map(item=>[item.occurrence_id,item])).values()].filter(item=>!(window.audit.staleJudgements&&item.occurrence_id==='occurrence-0'));
    return {total:250,confirmed:11+changed.length,pending:239-changed.length,groups:[{name:mergedName('大雨',runId),strategy:'直译',count:8},{name:mergedName('大雨点',runId),strategy:'强化',count:3},...changed.map(item=>({name:mergedName(item.group_name,runId),strategy:item.strategy??'',count:1}))],suggestions:window.audit.merges.some(merge=>merge.run_id===runId)?[]:[{left_group:'大雨点',right_group:'大雨',score:.75,reason:'共享文字'}]};
  };

  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
  window.__TAURI_INTERNALS__={ metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(fn){callbacks.set(++callbackId,fn);return callbackId;}, unregisterCallback(id){callbacks.delete(id);},invoke:async(cmd,args={})=>{
    calls.push({cmd,args});
    if(cmd==='load_app_settings') return null;
    if(cmd==='get_current_project') return structuredClone(view);
    if(cmd==='get_project_summary')return structuredClone(view.summary);
    if(cmd==='list_bookmarks')return [];
    if(cmd==='agent_projection')return {request_id:'projection',sequence:String(window.audit.sequence),data:{project:{project:view.project},binding_id:null,context:null,search_spec:{query:'',regex:false,case_sensitive:false,language_id:null},search_results:null,proposals:[],pending_ui_actions:[]}};
    if(cmd==='agent_call') {
      const {method,params}=args.call; let data={};
      if(method==='app.bind_session')data={binding_id:'binding',project_id:projectId,revision_id:view.project.current_revision_id};
      else if(method==='pipeline.list_proposals'||method==='pipeline.list')data=[];
      else if(method==='capabilities.get')data={contract_version:'2',generation:feature.generation,features:[structuredClone(feature)],slots:[],operators:[]};
      else if(method==='features.enable'||method==='features.retry_prepare'){Object.assign(feature,{desired_enabled:true,status:'ready',worker_state:'idle',generation:String(+feature.generation+1)});data=structuredClone(feature);}
      else if(method==='features.disable'||method==='features.cancel_prepare'){Object.assign(feature,{desired_enabled:false,status:'disabled',worker_state:'stopped',generation:String(+feature.generation+1)});data=structuredClone(feature);}
      else if(method==='features.update_preferences'){Object.assign(feature,params);data=structuredClone(feature);}
      else if(method==='pipeline.list_runs')data=structuredClone(window.audit.runs);
      else if(method==='research.start'){data={run_id:'run-'+(window.audit.runs.length+1),project_id:projectId,input_revision_id:'1',status:'completed',error:null,total:250,completed:250};window.audit.runs.unshift(data);}
      else if(method==='pipeline.get_run')data=structuredClone(window.audit.runs.find(run=>run.run_id===params.run_id));
      else if(method==='pipeline.read_result'){const rows=Array.from({length:250},(_,i)=>occurrence(i,params.run_id)).filter(item=>params.group_name===undefined||item.judgement?.group_name===params.group_name);data={run:structuredClone(window.audit.runs.find(run=>run.run_id===params.run_id)),items:rows.slice(params.cursor,params.cursor+params.limit),total:rows.length,next_cursor:params.cursor+params.limit<rows.length?params.cursor+params.limit:null};}
      else if(method==='research.summary')data=summaryFor(params.run_id);
      else if(method==='research.merge_groups'){window.audit.merges.push(params);data={committed_revision_id:'3'};}
      else if(method==='research.confirm'){if(window.audit.failSave)throw {code:'io_error',message:'research disk failure'};window.audit.confirmations.push(params);if(window.audit.commitConfirmRevision)window.audit.advanceRevision('research.confirm',false);data={committed_revision_id:view.project.current_revision_id};}
      return {request_id:args.call.request_id,sequence:String(window.audit.sequence),data};
    }
    if(cmd==='agent_runtime_status')return {configured:false,available:false,active_run_ids:[],recovered_interrupted_runs:0};
    if(cmd==='agent_runtime_history')return {session_id:'session',messages:[]};
    if(cmd==='plugin:event|listen'){(window.audit.listeners[args.event]??=[]).push(args.handler);return 1;}
    if(cmd.startsWith('plugin:event|'))return 1;
    if(cmd==='load_parallel_slice') {if(args.request.revision_id!==view.project.current_revision_id)throw {code:'stale_revision',message:'stale'};return {project_id:projectId,revision_id:view.project.current_revision_id,segments:args.request.segment_ids.map(id=>({segment_id:id,content:texts[id],content_hash:view.segments.find(s=>s.segment_id===id).content_hash}))};}
    if(cmd==='execute_command') {
      if(window.audit.failSave)throw {code:'io_error',message:'simulated disk failure'};
      const command=args.command;
      if(command.base_revision_id!==view.project.current_revision_id)throw {code:'stale_revision',message:'stale'};
      if(command.kind==='update_segment') {texts[command.payload.segment_id]=command.payload.content;const segment=view.segments.find(s=>s.segment_id===command.payload.segment_id);segment.content_hash=crypto.randomUUID();segment.content_length=command.payload.content.length;}
      const revision=String(BigInt(view.project.current_revision_id)+1n);view.project.current_revision_id=revision;view.summary.revision_id=revision;view.revisions.push({revision_id:revision,created_at:'2026-09-13T00:00:00Z',change_set:{operation:command.kind==='undo'?'undo:research.confirm':command.kind,affected_count:1},summary:'changed',state:'complete'});
      return {project_id:projectId,command_id:command.command_id,committed_revision_id:revision,status:'committed'};
    }
    if(cmd==='compare_revision')return {segment_changes:[],order_changes:[],alignment_changes:[]};
    if(cmd==='plugin:dialog|message')return window.audit.restoreApproved?'Ok':'Cancel';
    if(cmd==='plugin:dialog|open')return '/tmp/another.jm';
    if(cmd==='open_project')return structuredClone(view);
    return null;
  }};
});
try {
  await page.goto('http://127.0.0.1:1420');
  await page.getByText('中文正文 第0句',{exact:true}).waitFor();
  await page.locator('[data-nav-id="settings"]').click();
  await page.getByRole('navigation',{name:'设置分类'}).getByRole('button',{name:'译法研究',exact:true}).click();
  await page.locator('.research-settings input[type="checkbox"]').first().click();
  await page.getByRole('heading',{name:'译法研究',exact:true}).waitFor();
  await page.locator('.research-workspace:visible').waitFor();
  await page.getByRole('textbox',{name:'译法研究原文查询'}).fill('heavy~rain');
  await page.getByLabel('查询方式',{exact:true}).selectOption({label:'模糊匹配'});
  await page.getByLabel('模糊算法',{exact:true}).selectOption('fuzzy.edit_distance');
  const maxGap=page.getByLabel('最大词间隔',{exact:true});
  assert.equal(await maxGap.getAttribute('max'),'10');
  for(const invalid of ['11','-1','1.5','']) {
    await maxGap.fill(invalid);
    await page.locator('.research-query').evaluate(form=>form.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));
    await page.getByText('最大词间隔须为 0–10 的整数。',{exact:true}).waitFor();
    assert.equal(await page.evaluate(()=>window.audit.calls.filter(call=>call.args?.call?.method==='research.start').length),0,'Invalid gap never reaches Host');
  }
  await maxGap.fill('10');
  await page.getByRole('button',{name:'检索',exact:true}).click();
  await page.getByText('确认译文片段',{exact:true}).waitFor();
  assert.ok(await page.locator('.research-row').count()<40,'virtualization bounds DOM');
  const coverageStates=[
    ['partial','模型未覆盖完整上下文，请人工核查。'],
    ['no_links','未找到对应连线，这不表示省译。'],
    ['not_requested','未启用自动定位，请在译文中手工选择对应片段。'],
    ['context_missing','缺少对齐上下文，暂无法自动定位。'],
    ['unknown',null],['complete',null],['null',null],['missing',null],
  ];
  for(let index=0;index<coverageStates.length;index++) {
    const [coverage,message]=coverageStates[index];
    await page.locator(`.research-row[data-index="${index}"]`).click();
    if(message)await page.locator('.alignment-coverage').getByText(message,{exact:false}).waitFor();
    else assert.equal(await page.locator('.alignment-coverage').count(),0,coverage+' does not claim correctness or full coverage');
    assert.equal(await page.locator('.research-row.selected small').innerText(),'待确认');
    assert.ok(await page.getByLabel('译法组名',{exact:true}).isEnabled());
    if(coverage==='no_links') {
      await page.locator('.target-text [data-segment-content]').evaluate(element=>{
        const walker=document.createTreeWalker(element,NodeFilter.SHOW_TEXT);let text;
        while((text=walker.nextNode())&&!text.textContent.includes('今天有大雨')){}
        const range=document.createRange();range.setStart(text,3);range.setEnd(text,5);
        const selection=window.getSelection();selection.removeAllRanges();selection.addRange(range);
        element.dispatchEvent(new MouseEvent('mouseup',{bubbles:true}));
      });
      assert.equal(await page.getByLabel('译法组名',{exact:true}).inputValue(),'大雨','No links still supports manual target selection');
      await page.evaluate(()=>window.getSelection()?.removeAllRanges());
      await page.getByRole('button',{name:'放弃草稿',exact:true}).click();
    }
    assert.equal(await page.evaluate(()=>window.audit.confirmations.length),0,'Coverage state never records omission or confirmation automatically');
  }
  await page.locator('.research-viewport').evaluate(element=>{element.scrollTop=0;});
  await page.locator('.research-row[data-index="0"]').click();
  await page.getByLabel('译法组名',{exact:true}).fill('强降雨');
  await page.locator('[data-nav-id="parallel"]').click();
  const guard=page.getByRole('dialog',{name:'译法研究还有未保存草稿'});
  await guard.waitFor();
  await guard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.equal(await page.getByLabel('译法组名',{exact:true}).inputValue(),'强降雨');
  await page.evaluate(()=>window.audit.failSave=true);
  await page.getByRole('button',{name:'接受 · Enter',exact:true}).click();
  await page.getByText('research disk failure',{exact:false}).first().waitFor();
  assert.equal(await page.getByLabel('译法组名',{exact:true}).inputValue(),'强降雨');
  await page.evaluate(()=>window.audit.failSave=false);
  await page.getByRole('button',{name:'接受 · Enter',exact:true}).click();
  await page.getByText('已保存：译文片段 · 强降雨',{exact:false}).waitFor();
  assert.equal(await page.evaluate(()=>window.audit.confirmations[0].target_ranges[0].start_utf8),9);
  assert.equal(await page.evaluate(()=>window.audit.calls.find(call=>call.args?.call?.method==='research.start').args.call.params.max_gap),10);
  await page.evaluate(()=>{window.audit.staleJudgements=true;window.audit.advanceRevision('update_segment');});
  await page.locator('.research-row.selected small').getByText('待复核',{exact:true}).waitFor();
  await page.getByText('待复核：正文或对齐上下文已变化。',{exact:false}).waitFor();
  assert.equal(await page.getByLabel('译法组名',{exact:true}).inputValue(),'强降雨','Historical judgement remains available for review');
  await page.locator('.saved-status').getByText('历史判断（待复核）：',{exact:false}).waitFor();
  await page.getByText('全部命中 250 条 · 已确认 11 · 待确认 239',{exact:false}).waitFor();
  await page.evaluate(()=>{window.audit.staleJudgements=false;window.audit.advanceRevision('undo:update_segment');});
  await page.locator('.research-row.selected small').getByText('已确认',{exact:true}).waitFor();
  await page.getByText('全部命中 250 条 · 已确认 12 · 待确认 238',{exact:false}).waitFor();
  await maxGap.fill('0');
  await page.getByLabel('查询方式',{exact:true}).selectOption({label:'模糊匹配'});
  await page.getByLabel('模糊算法',{exact:true}).selectOption('fuzzy.char_ngram');
  await page.getByRole('button',{name:'检索',exact:true}).click();
  await page.waitForFunction(()=>window.audit.runs.length===2);
  assert.equal(await page.evaluate(()=>window.audit.calls.filter(call=>call.args?.call?.method==='research.start').at(-1).args.call.params.max_gap),0);
  assert.equal(await page.evaluate(()=>window.audit.calls.filter(c=>c.cmd==='agent_call'&&c.args.call.method==='research.start').at(-1).args.call.params.similarity_operator),'fuzzy.char_ngram');
  await page.getByLabel('译法组名',{exact:true}).fill('O');
  assert.equal(await page.evaluate(()=>window.audit.confirmations.length),1);
  await page.getByRole('region',{name:'译文确认，Enter 接受，O 省译，P 意译，E 返回编辑'}).focus();
  await page.keyboard.press('o');
  await page.waitForFunction(()=>window.audit.confirmations.length===2);
  const omission=await page.evaluate(()=>window.audit.confirmations.at(-1));
  assert.equal(omission.kind,'omission');assert.equal(omission.group_name,'省译');assert.deepEqual(omission.target_ranges,[]);
  await page.getByText('全部命中 250 条 · 已确认 12 · 待确认 238',{exact:false}).waitFor();
  await page.getByLabel('译法组名',{exact:true}).fill('未保存组名');
  await page.getByRole('button',{name:'合并',exact:true}).click();
  const mergeGuard=page.getByRole('dialog',{name:'译法研究还有未保存草稿'});await mergeGuard.waitFor();
  await mergeGuard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.audit.merges.length),0);
  await page.getByRole('button',{name:'放弃草稿',exact:true}).click();
  await page.getByRole('button',{name:'合并',exact:true}).click();
  await page.waitForFunction(()=>window.audit.merges.length===1);
  await page.getByText('直译 8 · 强化 3',{exact:false}).waitFor();
  await page.getByLabel('译法分布统计分母',{exact:true}).selectOption('all');
  await page.getByRole('button',{name:'大雨 4.4%，查看例句',exact:true}).click();
  await page.waitForFunction(()=>window.audit.calls.some(call=>call.args?.call?.method==='pipeline.read_result'&&call.args.call.params.group_name==='大雨'));
  await page.getByLabel('译法组名',{exact:true}).waitFor();
  await page.getByRole('button',{name:'清除组筛选',exact:true}).click();
  await page.getByLabel('策略编码',{exact:true}).fill('保留研究草稿');
  await page.evaluate(()=>{window.audit.merges=[];window.audit.emitRevision();});
  await page.getByText('工程版本已变化，当前研究草稿已保留；处理草稿后刷新结果',{exact:false}).waitFor();
  assert.equal(await page.getByLabel('策略编码',{exact:true}).inputValue(),'保留研究草稿');
  await page.getByRole('button',{name:'放弃草稿',exact:true}).click();
  await page.getByRole('button',{name:'大雨 3.2%，查看例句',exact:true}).waitFor();
  const historyCount=()=>page.evaluate(()=>window.audit.calls.filter(call=>call.cmd==='execute_command'&&['undo','redo','restore_revision'].includes(call.args.command.kind)).length);
  const toolbar=page.locator('.app-toolbar');
  await page.getByLabel('策略编码',{exact:true}).fill('撤销前保留');
  await toolbar.getByRole('button',{name:'撤销',exact:true}).click();
  await guard.waitFor();
  assert.equal(await historyCount(),0,'Undo waits for the research draft decision');
  await guard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.equal(await page.getByLabel('策略编码',{exact:true}).inputValue(),'撤销前保留');
  await page.evaluate(()=>window.audit.failSave=true);
  await toolbar.getByRole('button',{name:'撤销',exact:true}).click();
  await guard.getByRole('button',{name:'保存并继续',exact:true}).click();
  await page.getByText('research disk failure',{exact:false}).first().waitFor();
  assert.ok(await guard.isVisible());assert.equal(await historyCount(),0,'Failed save blocks Undo');
  assert.equal(await page.getByLabel('策略编码',{exact:true}).inputValue(),'撤销前保留');
  await page.evaluate(()=>{window.audit.failSave=false;window.audit.commitConfirmRevision=true;});
  const revisionBeforeSave=await page.evaluate(()=>window.audit.view.project.current_revision_id);
  await guard.getByRole('button',{name:'保存并继续',exact:true}).click();
  await page.waitForFunction(()=>window.audit.calls.some(call=>call.cmd==='execute_command'&&call.args.command.kind==='undo'));
  await page.getByText('已撤销并保存为新的 Revision',{exact:false}).waitFor();
  const undoCall=await page.evaluate(()=>window.audit.calls.find(call=>call.cmd==='execute_command'&&call.args.command.kind==='undo').args.command);
  assert.equal(undoCall.base_revision_id,String(BigInt(revisionBeforeSave)+1n),'Undo uses the Revision committed by saving, even without a revision event');
  await page.getByLabel('策略编码',{exact:true}).fill('重做前保留');
  await toolbar.getByRole('button',{name:'重做',exact:true}).click();
  await guard.waitFor();await guard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.equal(await historyCount(),1);assert.equal(await page.getByLabel('策略编码',{exact:true}).inputValue(),'重做前保留');
  await toolbar.getByRole('button',{name:'重做',exact:true}).click();
  await guard.getByRole('button',{name:'放弃草稿',exact:true}).click();
  await page.getByText('已重做并保存为新的 Revision',{exact:false}).waitFor();
  assert.equal(await historyCount(),2);
  await page.getByLabel('策略编码',{exact:true}).fill('切换历史前保留');
  await page.getByLabel('切换工作模式',{exact:true}).selectOption('history');
  await guard.waitFor();await guard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.ok(await page.locator('.research-workspace').isVisible());
  assert.equal(await page.getByLabel('策略编码',{exact:true}).inputValue(),'切换历史前保留');
  await page.getByLabel('切换工作模式',{exact:true}).selectOption('history');
  await guard.getByRole('button',{name:'放弃草稿',exact:true}).click();
  const restore=page.getByRole('button',{name:'恢复此版本',exact:true});await restore.waitFor();
  // Simulate a late draft update in the retained research component while History is active.
  await page.locator('.research-workspace').getByLabel('策略编码',{exact:true}).evaluate(input=>{input.value='历史恢复前保留';input.dispatchEvent(new Event('input',{bubbles:true}));});
  await restore.click();await guard.waitFor();
  assert.equal(await page.evaluate(()=>window.audit.calls.filter(call=>call.cmd==='plugin:dialog|message').length),0,'Draft decision precedes the native restore confirmation');
  await guard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.equal(await historyCount(),2);
  assert.equal(await page.locator('.research-workspace').getByLabel('策略编码',{exact:true}).inputValue(),'历史恢复前保留');
  await page.evaluate(()=>window.audit.restoreApproved=false);
  await restore.click();await guard.getByRole('button',{name:'放弃草稿',exact:true}).click();
  await page.waitForFunction(()=>window.audit.calls.some(call=>call.cmd==='plugin:dialog|message'));
  await restore.waitFor({state:'visible'});assert.equal(await historyCount(),2,'Native cancellation does not restore');
  await page.evaluate(()=>window.audit.restoreApproved=true);
  const historyBefore=await page.evaluate(()=>window.audit.view.revisions.map(revision=>revision.revision_id));
  await restore.click();
  await page.waitForFunction(()=>window.audit.calls.some(call=>call.cmd==='execute_command'&&call.args.command.kind==='restore_revision'));
  await page.waitForFunction(length=>window.audit.view.revisions.length===length+1,historyBefore.length);
  assert.deepEqual(await page.evaluate(()=>window.audit.view.revisions.slice(0,-1).map(revision=>revision.revision_id)),historyBefore,'Restore appends without deleting history');
  assert.equal(await page.getByLabel('切换工作模式',{exact:true}).inputValue(),'history','Restore preserves History mode');
  assert.equal(await page.evaluate(()=>window.audit.calls.find(call=>call.cmd==='execute_command'&&call.args.command.kind==='restore_revision').args.command.payload.target_revision_id),historyBefore.at(-2),'Restore uses the selected opaque Revision ID');
  await page.locator('[data-nav-id="parallel"]').click();
  await page.getByText('中文正文 第0句',{exact:true}).dblclick();
  const bodyEditor=page.locator('.segment-card__editor textarea');
  await bodyEditor.fill('正文撤销前的草稿');
  await toolbar.getByRole('button',{name:'撤销',exact:true}).click();
  const bodyGuard=page.getByRole('dialog',{name:'当前句段还有未保存编辑'});
  await bodyGuard.waitFor();assert.equal(await historyCount(),3);
  await bodyGuard.getByRole('button',{name:'继续编辑',exact:true}).click();
  assert.equal(await bodyEditor.inputValue(),'正文撤销前的草稿');
  await toolbar.getByRole('button',{name:'撤销',exact:true}).click();
  await bodyGuard.getByRole('button',{name:'放弃草稿',exact:true}).click();
  await page.getByText('已撤销并保存为新的 Revision',{exact:false}).waitFor();
  assert.equal(await historyCount(),4);
  assert.equal(await page.getByLabel('切换工作模式',{exact:true}).inputValue(),'review');
  await page.locator('[data-nav-id="research"]').click();
  await page.getByRole('button',{name:'功能设置',exact:true}).click();
  await page.locator('.research-settings input[type="checkbox"]').first().click();
  await page.locator('[data-nav-id="research"]').click();
  await page.getByText('功能尚未就绪。已有结果仍可查看',{exact:false}).waitFor();
  assert.ok(await page.getByRole('button',{name:'检索',exact:true}).isDisabled());
  assert.ok(await page.locator('.research-row').count()>0);
  assert.ok(await page.getByRole('button',{name:'接受 · Enter',exact:true}).isDisabled());
  assert.ok(await page.getByLabel('译法组名',{exact:true}).isDisabled());
  if (process.env.RESEARCH_SCREENSHOT) await page.screenshot({path:process.env.RESEARCH_SCREENSHOT});
  await page.goto('http://127.0.0.1:1420/?research_history_fixture=1');
  await page.locator('[data-nav-id="research"]').waitFor();
  await page.locator('[data-nav-id="research"]').click();
  await page.getByText('确认译文片段',{exact:true}).waitFor();
  assert.equal(await page.getByLabel('历史查询',{exact:true}).inputValue(),'restored-run');
  assert.ok(await page.getByRole('button',{name:'检索',exact:true}).isDisabled());
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({result:'actual Vue UI passed',checks:['single-toggle activation','automatic navigation','bounded result DOM','dirty leave/cancel guard','confirmation failure preserves draft','successful typed UTF8 confirmation','both fuzzy provider IDs transmitted','input O is inert and scoped O records omission without ranges','feature disable keeps historical result and blocks new runs','cold-start discovers saved runs while feature remains disabled','whole-run statistics and explicit denominator','merge uses draft guard and preserves strategy categories','group selection requests filtered bounded results','canonical revision refresh preserves dirty research then reloads restored groups','Undo cancel/save failure retains research draft','save before Undo refreshes the committed Revision','Redo requires an explicit draft decision','mode dropdown guards research draft','Restore guards retained draft and preserves append-only History','body Edit uses its existing draft guard before Undo','gap bounds 0–10 validated before Host submission','context changes mark saved judgements needs-review and exclude them from confirmation counts','alignment coverage distinguishes partial/no-links/not-requested/context-missing while retaining manual selection']}));
} catch(error) { if (process.env.RESEARCH_FAILURE_SCREENSHOT) await page.screenshot({path:process.env.RESEARCH_FAILURE_SCREENSHOT}); console.error(await page.locator('body').innerText()); throw error; } finally { await browser.close(); }
