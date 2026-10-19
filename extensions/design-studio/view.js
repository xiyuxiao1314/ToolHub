(() => {
  'use strict';
  const DS=globalThis.DesignStudio,root=document.getElementById('design-studio'),find=id=>document.getElementById(id);
  let config=DS.clone(DS.defaultConfig),presets=[],revision=0,loaded=false,writable=false,output='json',dispose=()=>{},saveQueue=Promise.resolve(),autosave;
  const moduleOrder=['hero','search','filters','content','footer'];
  const symbols={grid:'▦',metro:'▰',masonry:'▥',list:'☷',canvas:'⌘',kanban:'▤',sidebar:'▕',rail:'⊞',top:'▔',none:'□',clean:'○',glass:'◒',editorial:'Aa',brutalist:'▣'};
  const sources=[['Windows 布局基础','https://learn.microsoft.com/en-us/windows/apps/design/basics/'],['无限画布参考','https://tldraw.dev/examples/basic'],['布局与共享元素动画','https://motion.dev/docs/react-layout-animations'],['虚拟列表参考','https://tanstack.com/virtual/latest/docs/introduction'],['Android 自适应主从布局','https://developer.android.com/develop/ui/compose/layouts/adaptive/list-detail']];
  function message(text,error=false){find('studio-message').hidden=!text;find('studio-message').textContent=text;find('studio-message').classList.toggle('is-error',error);}
  function persist(){
    if(!loaded||!writable)return Promise.resolve(false);
    const snapshot={schema:'toolhub.design-presets/v1',presets:DS.clone(presets),last:DS.clone(config)};
    find('storage-status').textContent='正在保存…';
    saveQueue=saveQueue.then(async()=>{const result=await toolhub.storage.set(snapshot,revision);revision=result.revision;find('storage-status').textContent='已保存到本机';return true;}).catch(error=>{writable=false;find('storage-status').textContent='保存失败，当前内容仍可复制';message(String(error.message||error)+'；请先复制配置，重新打开页面后再保存。',true);return false;});
    return saveQueue;
  }
  function scheduleSave(){clearTimeout(autosave);if(loaded&&writable)autosave=setTimeout(persist,450);}
  function presetOptions(selected){
    const select=find('preset-select');select.replaceChildren();
    for(const [label,rows,prefix]of [['内置预设',DS.presets,'builtin:'],['我的预设',presets,'user:']]){if(!rows.length)continue;const group=document.createElement('optgroup');group.label=label;rows.forEach(preset=>{const option=document.createElement('option');option.value=prefix+preset.id;option.textContent=preset.name;group.append(option);});select.append(group);}
    if(selected&&[...select.options].some(option=>option.value===selected))select.value=selected;
    find('delete-preset').disabled=!select.value.startsWith('user:');
  }
  function library(){
    const container=find('library-content');container.replaceChildren();
    for(const [key,label]of [['layout','页面结构'],['navigation','导航结构'],['appearance','视觉语言']]){
      const section=document.createElement('details');section.className='library-section';section.open=true;
      const summary=document.createElement('summary');summary.textContent=label;const choices=document.createElement('div');choices.className='library-choices';
      for(const [value,name]of Object.entries(DS.lists[key])){const button=document.createElement('button');button.type='button';button.className='library-choice';button.setAttribute('aria-pressed',String(config[key]===value));const symbol=document.createElement('span');symbol.className='choice-symbol';symbol.textContent=symbols[value];symbol.setAttribute('aria-hidden','true');const text=document.createElement('span');text.textContent=name;button.append(symbol,text);button.addEventListener('click',()=>{config[key]=value;const normalized=DS.normalize(config);config=normalized.config;message(normalized.notes.join(' '));render(true);});choices.append(button);}section.append(summary,choices);container.append(section);
    }
    const effects=document.createElement('details');effects.className='library-section';effects.open=true;const summary=document.createElement('summary');summary.textContent='交互与动态效果';const effectRows=document.createElement('div');effectRows.className='library-effects';
    for(const [key,name]of Object.entries(DS.lists.effects)){const label=document.createElement('label');const checkbox=document.createElement('input');checkbox.type='checkbox';checkbox.checked=config.effects.includes(key);checkbox.setAttribute('aria-label',name);checkbox.disabled=key==='drag'&&(config.layout==='canvas'||config.performance==='virtual');checkbox.addEventListener('change',()=>{config.effects=checkbox.checked?[...config.effects,key]:config.effects.filter(id=>id!==key);render(true);});label.append(checkbox,document.createTextNode(name));effectRows.append(label);}effects.append(summary,effectRows);container.append(effects);
    const performance=document.createElement('details');performance.className='library-section';performance.open=true;const title=document.createElement('summary');title.textContent='性能策略';const select=document.createElement('select');select.setAttribute('aria-label','性能策略');select.style.width='100%';
    Object.entries(DS.lists.performance).forEach(([value,name])=>{const option=document.createElement('option');option.value=value;option.textContent=name;option.disabled=(value==='virtual'&&config.layout!=='list')||(value==='culling'&&config.layout!=='canvas');select.append(option);});select.value=config.performance;select.addEventListener('change',()=>{config.performance=select.value;const next=DS.normalize(config);config=next.config;message(next.notes.join(' '));render(true);});const help=document.createElement('p');help.className='library-caption';help.textContent='虚拟列表用于主从列表；视口裁剪用于画布。布局切换会说明相关调整。';performance.append(title,select,help);container.append(performance);
    find('effect-count').textContent='23 项选择';
  }
  function modules(){
    const container=find('module-order');container.replaceChildren();const ordered=[...config.modules,...moduleOrder.filter(id=>!config.modules.includes(id))];
    ordered.forEach(id=>{const row=document.createElement('div');row.className='module-row';const label=document.createElement('label'),checkbox=document.createElement('input');checkbox.type='checkbox';checkbox.checked=config.modules.includes(id);checkbox.disabled=id==='content';checkbox.setAttribute('aria-label','模块：'+DS.lists.modules[id]);checkbox.addEventListener('change',()=>{config.modules=checkbox.checked?[...config.modules,id]:config.modules.filter(value=>value!==id);render(true);});label.append(checkbox,document.createTextNode(DS.lists.modules[id]));row.append(label);
      for(const [shift,symbol,name]of [[-1,'↑','上移'],[1,'↓','下移']]){const button=document.createElement('button');button.type='button';button.textContent=symbol;button.setAttribute('aria-label',name+DS.lists.modules[id]);const index=config.modules.indexOf(id);button.disabled=index<0||index+shift<0||index+shift>=config.modules.length;button.addEventListener('click',()=>{const other=config.modules[index+shift];config.modules[index+shift]=id;config.modules[index]=other;render(true);});row.append(button);}container.append(row);
    });
  }
  function controls(){
    const values={'project-title':config.title,'project-purpose':config.purpose,'target-platform':config.platform,viewport:config.viewport,columns:String(config.columns),density:config.density,radius:String(config.radius),gap:String(config.gap),accent:config.accent,duration:String(config.duration),'data-size':String(config.dataSize)};
    for(const [id,value]of Object.entries(values)){const element=find(id);if(document.activeElement!==element)element.value=value;}
    find('radius-value').textContent=config.radius+' px';find('gap-value').textContent=config.gap+' px';find('data-size').querySelector('[value="10000"]').disabled=config.performance!=='virtual';find('columns').disabled=!['grid','metro','masonry'].includes(config.layout);find('dark-preview').checked=config.dark;
  }
  const outputNotes={json:'JSON 只描述设计选择，不包含项目业务数据或可执行导入代码。',html:'复制保存为 .html 后可在浏览器直接打开；包含同一套示例渲染与交互。Android / 桌面仍需适配各自技术栈。',agent:'说明包含当前结构、效果、模块顺序、参数与验收要求；由 Agent 结合真实项目实现。',sources:'以下是设计与技术参考。演示为自行编写，不捆绑这些第三方库，也不宣称已提供完整设计工具。'};
  function updateOutput(){
    let text=JSON.stringify(config,null,2);
    if(output==='html')text=DS.exportHtml(config,globalThis.DESIGN_PREVIEW_CSS);
    if(output==='agent')text=DS.agentPrompt(config);
    if(output==='sources')text=sources.map(([name,url])=>name+'\n'+url).join('\n\n')+'\n\n本版边界：\n- 效果库为有限的声明式组合，不执行任意导入脚本。\n- 无限画布支持平移、缩放、复位与视口裁剪，不含节点编辑与协作。\n- 虚拟列表是本地示例，不代表已接入分页接口。\n- 拖拽排序仅改变预览内容顺序，不改变设计模块或真实项目数据。\n- 预设保存在本机 WebView；清除该配置文件会重置。请复制 JSON 备份。';
    find('output-text').value=text;find('output-note').textContent=outputNotes[output];find('output-label').textContent={json:'当前组合的配置代码',html:'完整 HTML 示例代码',agent:'可直接交给 Agent 的设计说明',sources:'参考来源与当前实现边界'}[output];
    root.querySelectorAll('[data-output]').forEach(button=>button.setAttribute('aria-selected',String(button.dataset.output===output)));
  }
  function render(save){
    const focusLabel=document.activeElement?.getAttribute('aria-label');
    dispose();library();modules();controls();
    find('preview-container').classList.toggle('is-mobile',config.viewport==='mobile');find('preview-container').classList.toggle('is-tablet',config.viewport==='tablet');
    dispose=DS.render(find('studio-export'),config);find('preview-title').textContent=DS.lists.layout[config.layout];find('preview-summary').textContent=DS.lists.navigation[config.navigation]+' · '+DS.lists.appearance[config.appearance]+' · '+DS.lists.performance[config.performance];
    find('combination-notes').replaceChildren();const title=document.createElement('strong');title.textContent='组合说明：';const note=document.createTextNode(config.performance==='virtual'?'动态高度虚拟列表保留展开状态，只挂载视口与缓冲区。':config.layout==='canvas'?'拖动空白处平移画布，使用缩放 / 复位按钮；按钮获焦后可用方向键平移。':config.effects.includes('drag')?'拖动卡片调整预览顺序；看板支持跨列移动。':'点击卡片试用选择 / 展开，搜索和分类会筛选示例项目。');find('combination-notes').append(title,note);updateOutput();if(focusLabel&&document.activeElement===document.body){[...root.querySelectorAll('[aria-label]')].find(element=>element.getAttribute('aria-label')===focusLabel)?.focus({preventScroll:true});}if(save)scheduleSave();
  }
  find('studio-export').addEventListener('design-preview-status',event=>find('preview-info').textContent=event.detail);
  for(const [id,key]of [['project-title','title'],['project-purpose','purpose'],['target-platform','platform'],['viewport','viewport'],['columns','columns'],['density','density'],['radius','radius'],['gap','gap'],['accent','accent'],['duration','duration'],['data-size','dataSize']]){
    const element=find(id);const change=()=>{config[key]=['columns','radius','gap','duration','dataSize'].includes(key)?Number(element.value):element.value;if((key==='title'||key==='purpose')&&!element.value.trim())return;const next=DS.normalize(config);config=next.config;message(next.notes.join(' '));render(true);};
    element.addEventListener(['project-title','project-purpose','radius','gap','accent'].includes(id)?'input':'change',change);
    if(id==='project-title'||id==='project-purpose')element.addEventListener('blur',()=>{if(!element.value.trim()){config[key]=DS.defaultConfig[key];render(true);}});
  }
  find('preset-select').addEventListener('change',()=>find('delete-preset').disabled=!find('preset-select').value.startsWith('user:'));
  find('dark-preview').addEventListener('change',()=>{config.dark=find('dark-preview').checked;render(true);});
  find('apply-preset').addEventListener('click',()=>{const value=find('preset-select').value,rows=value.startsWith('user:')?presets:DS.presets,preset=rows.find(item=>item.id===value.slice(value.indexOf(':')+1));if(preset){config=DS.clone(preset.config);render(true);message('已应用「'+preset.name+'」，可以继续自由调整。');}});
  find('save-preset').addEventListener('click',async()=>{const name=find('preset-name').value.trim();if(!name){find('preset-name').focus();message('请先为当前组合填写一个预设名称。');return;}if(!writable){message('当前无法保存；请先复制 JSON 备份。',true);return;}if(presets.some(preset=>preset.name===name)){message('已有同名预设，请使用新的名称，避免覆盖。',true);return;}if(presets.length>=24){message('最多保存 24 个自定义预设，请先移除不用的预设。',true);return;}clearTimeout(autosave);const preset={id:crypto.randomUUID(),name,config:DS.validate(config)};presets.push(preset);if(await persist()){presetOptions('user:'+preset.id);message('预设「'+name+'」已保存；重新打开设计台后仍可使用。');}});
  let pendingDelete='';find('delete-preset').addEventListener('click',async()=>{const value=find('preset-select').value;if(!value.startsWith('user:'))return;const id=value.slice(5),preset=presets.find(item=>item.id===id);if(!preset)return;if(pendingDelete!==id){pendingDelete=id;message('再次点击“删除自定义预设”可删除「'+preset.name+'」。当前设计组合会保留。');return;}clearTimeout(autosave);presets=presets.filter(item=>item.id!==id);if(await persist()){presetOptions();pendingDelete='';message('已删除预设，当前设计仍保留。');}});
  root.querySelectorAll('[data-output]').forEach(button=>button.addEventListener('click',()=>{output=button.dataset.output;updateOutput();}));
  const tabs=[...root.querySelectorAll('[data-output]')];tabs.forEach((tab,index)=>tab.addEventListener('keydown',event=>{const offset=event.key==='ArrowRight'?1:event.key==='ArrowLeft'?-1:0;if(offset){event.preventDefault();const next=tabs[(index+offset+tabs.length)%tabs.length];next.focus();next.click();}}));
  find('copy-output').addEventListener('click',()=>{const text=find('output-text');text.focus();text.select();try{if(document.execCommand('copy')){message('已复制'+{json:'配置 JSON',html:'HTML 示例',agent:'Agent 设计说明',sources:'参考说明'}[output]);return;}}catch{}message('内容已全选，请按 Ctrl+C（macOS 使用 Command+C）复制。');});
  find('replay-motion').addEventListener('click',()=>{dispose();dispose=DS.render(find('studio-export'),config);message(config.duration?'已重播当前组合的入场效果。':'动效已关闭，可在参数中调整。');});
  let previousFocus;const closeImport=()=>{find('import-panel').hidden=true;previousFocus?.focus();};
  find('import-config').addEventListener('click',()=>{previousFocus=document.activeElement;find('import-panel').hidden=false;find('import-error').textContent='';find('import-text').focus();});find('cancel-import').addEventListener('click',closeImport);
  find('confirm-import').addEventListener('click',()=>{try{const text=find('import-text').value;if(new TextEncoder().encode(text).length>32768)throw Error('配置最多 32 KiB');config=DS.validate(JSON.parse(text));closeImport();render(true);message('配置已校验并应用；未执行导入代码。');}catch(error){find('import-error').textContent=String(error.message||error);}});
  find('import-panel').addEventListener('keydown',event=>{if(event.key==='Escape'){event.preventDefault();closeImport();}if(event.key==='Tab'){const focusable=[...find('import-panel').querySelectorAll('textarea,button')],first=focusable[0],last=focusable[focusable.length-1];if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}}});
  presetOptions();render(false);
  (async()=>{try{if(!globalThis.toolhub?.storage)throw Error('当前宿主缺少扩展预设存储，请使用 ToolHub beta.6 或更新版本');const stored=await toolhub.storage.get();revision=stored.revision;if(stored.data!==null){const data=stored.data;if(data.schema!=='toolhub.design-presets/v1'||!Array.isArray(data.presets)||data.presets.length>24||Object.keys(data).some(key=>!['schema','presets','last'].includes(key)))throw Error('本机预设格式需要核对，原数据未覆盖');const ids=new Set();presets=data.presets.map(preset=>{if(!preset||Object.keys(preset).some(key=>!['id','name','config'].includes(key))||typeof preset.id!=='string'||preset.id.length>64||ids.has(preset.id)||typeof preset.name!=='string'||!preset.name.trim()||preset.name.length>40)throw Error('本机预设条目无效，原数据未覆盖');ids.add(preset.id);return {...preset,config:DS.validate(preset.config)};});config=DS.validate(data.last);}loaded=true;writable=true;presetOptions();render(false);find('storage-status').textContent='本机预设已就绪';}catch(error){loaded=true;writable=false;find('storage-status').textContent='仅本次会话，可复制配置';message(String(error.message||error),true);}})();
})();
