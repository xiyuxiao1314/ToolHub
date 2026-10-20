(() => {
  'use strict';
  const DS=globalThis.DesignStudio,root=document.getElementById('design-studio'),find=id=>document.getElementById(id);
  let config=DS.clone(DS.defaultConfig),presets=[],revision=0,loaded=false,writable=false,output='json',dispose=()=>{},saveQueue=Promise.resolve(),autosave;
  let step=1,choice='none',appliedChoice='',hasDraft=false;
  const emptyConfig=DS.validate({...DS.clone(DS.defaultConfig),title:'未命名设计',purpose:'我的应用',layout:'grid',navigation:'none',appearance:'clean',contentStyle:'projects',canvasMode:'scatter',effects:[],performance:'standard',modules:['content'],columns:3,radius:8,gap:16,accent:'#2563eb',duration:240,dataSize:12,dark:false});
  const moduleOrder=['hero','search','filters','content','footer'];
  const symbols={grid:'▦',metro:'▰',masonry:'▥',list:'☷',spotlight:'◨',filmstrip:'▱',index:'≡',bento:'⊞',canvas:'⌘',kanban:'▤',sidebar:'▕',rail:'⊞',top:'▔',none:'□',immersive:'◐',covers:'◧',typographic:'Aa',projects:'▤',clean:'○',glass:'◒',editorial:'Aa',brutalist:'▣'};
  const sources=[['Awwwards / 动态网格','https://www.awwwards.com/websites/dynamic-grid-layout-examples/'],['Siteinspire / 编辑与字体设计','https://www.siteinspire.com/'],['Godly / 多风格界面灵感','https://godly.website/'],['Codrops / 2026 Creative Hub','https://tympanus.net/codrops/hub/'],['Codrops / Atelier North','https://atelier-north.crnacura.workers.dev/'],['Codrops / 2026 画廊转场','https://tympanus.net/Tutorials/InfiniteScrollGSAPGallery/'],['Codrops / Infinite Layers Grid','https://tympanus.net/Tutorials/InfiniteLayersGrid/'],['Codrops / Grid to Fullscreen','https://tympanus.net/Tutorials/GridToFullscreenAnimations/'],['Folia：沉浸式封面与网格交互参考（未复用源码或封面）','https://github.com/chthollyphile/folia-major'],['Windows 布局基础','https://learn.microsoft.com/en-us/windows/apps/design/basics/'],['无限画布参考','https://tldraw.dev/examples/basic'],['布局与共享元素动画','https://motion.dev/docs/react-layout-animations'],['虚拟列表参考','https://tanstack.com/virtual/latest/docs/introduction'],['Android 自适应主从布局','https://developer.android.com/develop/ui/compose/layouts/adaptive/list-detail']];
  const directions={
    metro:['expressive','沉浸媒体 / 音乐 / 素材墙','Folia · 大小磁贴与内容层次','https://github.com/chthollyphile/folia-major','让内容形成主次，用窄缝、大封面与克制的界面营造沉浸感。'],
    canvas:['spatial','灵感采集 / 素材探索 / 视觉档案','Codrops · Infinite Layers Grid','https://tympanus.net/Tutorials/InfiniteLayersGrid/','内容延伸到视口之外，以世界坐标组织拼贴，只绘制可见节点。'],
    gallery:['expressive','艺术档案 / 建筑 / 生活方式','Codrops · Atelier North / 建筑编辑结构','https://atelier-north.crnacura.workers.dev/','纸感底色、衬线标题、不等高图像，以阅读节奏代替卡片堆叠。'],
    portfolio:['expressive','独立创作者 / 实验室 / 活动','Siteinspire · 字体与非惯常布局','https://www.siteinspire.com/','用大字、酸性色块、硬边框线制造有意识的冲突与节奏。'],
    creator:['functional','项目管理 / 桌面工具 / 工作台','Windows · 自适应布局原则','https://learn.microsoft.com/en-us/windows/apps/design/basics/','降低装饰密度，用对齐、行距和清晰的状态组织高频工作。'],
    library:['functional','万条素材 / 项目目录 / 资源检索','TanStack · 虚拟列表原理','https://tanstack.com/virtual/latest/docs/introduction','主从结构与真实虚拟列表，渲染条数可见，展开状态独立保存。'],
    kanban:['functional','创意生产 / 任务 / 团队协作','Windows · Fluent 布局原则','https://learn.microsoft.com/en-us/windows/apps/design/basics/','透明层次与柔和边界组织任务，支持跨列拖动。'],
    mobile:['mobile','Android / 随身灵感 / 收藏','Android · 自适应设计','https://developer.android.com/develop/ui/compose/layouts/adaptive/list-detail','考虑拇指操作、底部导航、安全区与触控目标，不只是把网页缩窄。'],
    cinema:['expressive','品牌故事 / 影视 / 单项目展示','Codrops · Grid to Fullscreen','https://tympanus.net/Tutorials/GridToFullscreenAnimations/','大幅图像配少量文字，水平切幕与展开转场形成叙事节奏。'],
    swiss:['expressive','设计事务所 / 档案 / 作品索引','Siteinspire · 网格与字体','https://www.siteinspire.com/','编号、横线、大字与小型图像建立严谨的编辑秩序。'],
    photo:['expressive','摄影 / 旅行 / 作品册','Codrops · 2026 画廊与详情转场','https://tympanus.net/Tutorials/InfiniteScrollGSAPGallery/','横向留白与不同的封面节奏，形成可滚动的长幅画册。'],
    product:['functional','产品首页 / 能力总览 / SaaS','Godly · 界面与品牌设计','https://godly.website/','突出一个主能力与一组辅助能力，用不等宽分区而非等权卡片。']
  };
  function message(text,error=false){find('studio-message').hidden=!text;find('studio-message').textContent=text;find('studio-message').classList.toggle('is-error',error);}
  function persist(){
    if(!loaded||!writable)return Promise.resolve(false);
    const snapshot={schema:'toolhub.design-presets/v1',presets:DS.clone(presets),last:DS.clone(config)};
    find('storage-status').textContent='正在保存…';
    saveQueue=saveQueue.then(async()=>{const result=await toolhub.storage.set(snapshot,revision);revision=result.revision;find('storage-status').textContent='已保存到本机';return true;}).catch(error=>{writable=false;find('storage-status').textContent='保存失败，当前内容仍可复制';message(String(error.message||error)+'；请先复制配置，重新打开页面后再保存。',true);return false;});
    return saveQueue;
  }
  function scheduleSave(){clearTimeout(autosave);if(loaded&&writable)autosave=setTimeout(persist,450);}
  function library(){
    const container=find('library-content');container.replaceChildren();
    for(const [key,label]of [['layout','页面结构'],['navigation','导航结构'],['appearance','视觉语言'],['contentStyle','内容语言']]){
      const section=document.createElement('details');section.className='library-section';section.open=true;
      const summary=document.createElement('summary');summary.textContent=label;const choices=document.createElement('div');choices.className='library-choices';
      for(const [value,name]of Object.entries(DS.lists[key])){const button=document.createElement('button');button.type='button';button.className='library-choice';button.setAttribute('aria-label',name);button.setAttribute('aria-pressed',String(config[key]===value));const symbol=document.createElement('span');symbol.className='choice-symbol';symbol.textContent=symbols[value];symbol.setAttribute('aria-hidden','true');const text=document.createElement('span');text.textContent=name;button.append(symbol,text);button.addEventListener('click',()=>{config[key]=value;const normalized=DS.normalize(config);config=normalized.config;message(normalized.notes.join(' '));render(true);});choices.append(button);}section.append(summary,choices);container.append(section);
    }
    const effects=document.createElement('details');effects.className='library-section';effects.open=true;const summary=document.createElement('summary');summary.textContent='交互与动态效果';const effectRows=document.createElement('div');effectRows.className='library-effects';
    for(const [key,name]of Object.entries(DS.lists.effects)){const label=document.createElement('label');const checkbox=document.createElement('input');checkbox.type='checkbox';checkbox.checked=config.effects.includes(key);checkbox.setAttribute('aria-label',name);checkbox.disabled=key==='drag'&&(config.layout==='canvas'||config.performance==='virtual');checkbox.addEventListener('change',()=>{config.effects=checkbox.checked?[...config.effects,key]:config.effects.filter(id=>id!==key);render(true);});label.append(checkbox,document.createTextNode(name));effectRows.append(label);}effects.append(summary,effectRows);container.append(effects);
    const performance=document.createElement('details');performance.className='library-section';performance.open=true;const title=document.createElement('summary');title.textContent='性能策略';const select=document.createElement('select');select.setAttribute('aria-label','性能策略');select.style.width='100%';
    Object.entries(DS.lists.performance).forEach(([value,name])=>{const option=document.createElement('option');option.value=value;option.textContent=name;option.disabled=(value==='virtual'&&config.layout!=='list')||(value==='culling'&&config.layout!=='canvas');select.append(option);});select.value=config.performance;select.addEventListener('change',()=>{config.performance=select.value;const next=DS.normalize(config);config=next.config;message(next.notes.join(' '));render(true);});performance.append(title,select);container.append(performance);
    find('effect-count').textContent='结构 × 内容 × 动效';
  }
  function modules(){
    const container=find('module-order');container.replaceChildren();const ordered=[...config.modules,...moduleOrder.filter(id=>!config.modules.includes(id))];
    ordered.forEach(id=>{const row=document.createElement('div');row.className='module-row';const label=document.createElement('label'),checkbox=document.createElement('input');checkbox.type='checkbox';checkbox.checked=config.modules.includes(id);checkbox.disabled=id==='content';checkbox.setAttribute('aria-label','模块：'+DS.lists.modules[id]);checkbox.addEventListener('change',()=>{config.modules=checkbox.checked?[...config.modules,id]:config.modules.filter(value=>value!==id);render(true);});label.append(checkbox,document.createTextNode(DS.lists.modules[id]));row.append(label);
      for(const [shift,symbol,name]of [[-1,'↑','上移'],[1,'↓','下移']]){const button=document.createElement('button');button.type='button';button.textContent=symbol;button.setAttribute('aria-label',name+DS.lists.modules[id]);const index=config.modules.indexOf(id);button.disabled=index<0||index+shift<0||index+shift>=config.modules.length;button.addEventListener('click',()=>{const other=config.modules[index+shift];config.modules[index+shift]=id;config.modules[index]=other;render(true);});row.append(button);}container.append(row);
    });
  }
  function controls(){
    const values={'project-title':config.title,'project-purpose':config.purpose,'target-platform':config.platform,'canvas-mode':config.canvasMode,viewport:config.viewport,columns:String(config.columns),density:config.density,radius:String(config.radius),gap:String(config.gap),accent:config.accent,duration:String(config.duration),'data-size':String(config.dataSize)};
    for(const [id,value]of Object.entries(values)){const element=find(id);if(document.activeElement!==element)element.value=value;}
    find('radius-value').textContent=config.radius+' px';find('gap-value').textContent=config.gap+' px';find('data-size').querySelector('[value="10000"]').disabled=config.performance!=='virtual';find('columns').disabled=!['grid','metro','masonry'].includes(config.layout);find('dark-preview').checked=config.dark;find('canvas-mode').disabled=config.layout!=='canvas';
  }
  const outputNotes={json:'JSON 只描述设计选择，不包含项目业务数据或可执行导入代码。',html:'复制保存为 .html 后可在浏览器直接打开；包含同一套示例渲染与交互。Android / 桌面仍需适配各自技术栈。',agent:'说明包含当前结构、效果、模块顺序、参数与验收要求；由 Agent 结合真实项目实现。',sources:'以下是设计与技术参考。演示为自行编写，不捆绑这些第三方库，也不宣称已提供完整设计工具。'};
  function activeDirection(){const id=appliedChoice.replace('builtin:',''),p=DS.presets.find(p=>p.id===id);return p&&['layout','appearance','contentStyle','platform'].every(key=>p.config[key]===config[key])?directions[id]:null;}
  function updateOutput(){
    let text=JSON.stringify(config,null,2);
    if(output==='html')text=DS.exportHtml(config,globalThis.DESIGN_PREVIEW_CSS);
    if(output==='agent'){text=DS.agentPrompt(config);const direction=activeDirection();if(direction)text+='\n\n设计思路：'+direction[4]+'\n参考：'+direction[2]+' / '+direction[3]+'\n参考用于理解设计原则，使用项目自有或获授权的内容。';}
    if(output==='sources')text=sources.map(([name,url])=>name+'\n'+url).join('\n\n')+'\n\n本版边界：\n- 效果库为有限的声明式组合，不执行任意导入脚本。\n- 无限画布支持平移、缩放、复位与视口裁剪，不含节点编辑与协作。\n- 虚拟列表是本地示例，不代表已接入分页接口。\n- 拖拽排序仅改变预览内容顺序，不改变设计模块或真实项目数据。\n- 预设保存在本机 WebView；清除该配置文件会重置。请复制 JSON 备份。';
    find('output-text').value=text;find('output-note').textContent=outputNotes[output];find('output-label').textContent={json:'当前组合的配置代码',html:'完整 HTML 示例代码',agent:'可直接交给 Agent 的设计说明',sources:'参考来源与当前实现边界'}[output];
    root.querySelectorAll('[data-output]').forEach(button=>button.setAttribute('aria-selected',String(button.dataset.output===output)));
  }
  function collection(){
    const container=find('design-collection'),filter=find('collection-filter').value;container.replaceChildren();
    const rows=[{id:'none',name:'无预设',config:emptyConfig,category:'all'},...(hasDraft?[{id:'draft',name:'继续当前设计',config,category:'all'}]:[]),...DS.presets.map(p=>({...p,id:'builtin:'+p.id,category:directions[p.id][0]})),...presets.map(p=>({...p,id:'user:'+p.id,category:'user'}))];
    for(const preset of rows){if(filter!=='all'&&preset.category!==filter&&preset.id!=='none'&&preset.id!=='draft')continue;
      const button=document.createElement('button');button.type='button';button.className='direction-card d-'+preset.id.replace('builtin:','');button.dataset.choice=preset.id;button.setAttribute('aria-pressed',String(choice===preset.id));button.disabled=!loaded;button.setAttribute('aria-label',preset.name);
      const mini=document.createElement('span');mini.className='direction-mini';mini.setAttribute('aria-hidden','true');
      if(preset.id==='none')mini.textContent='＋';else if(preset.id==='draft')mini.textContent='↗';else for(let i=0;i<6;i++){const tile=document.createElement('i');if(preset.config.contentStyle==='typographic')tile.textContent=['A','a','字','R','M','02'][i];mini.append(tile);}
      const name=document.createElement('strong');name.textContent=preset.name;button.append(mini,name);button.addEventListener('click',()=>{choice=preset.id;container.querySelectorAll('button').forEach(row=>row.setAttribute('aria-pressed',String(row===button)));updateSteps();});container.append(button);
    }
  }
  function updateSteps(){
    root.dataset.step=String(step);root.querySelectorAll('[data-step-panel]').forEach(panel=>panel.hidden=Number(panel.dataset.stepPanel)!==step);
    root.querySelectorAll('[data-step]').forEach(button=>{const active=Number(button.dataset.step)===step;button.setAttribute('aria-current',active?'step':'false');button.disabled=!loaded;});
    find('step-back').hidden=step===1;find('step-next').hidden=step===3;find('step-next').disabled=!loaded;find('step-next').textContent=step===1?'下一步：预览微调 →':'下一步：导出 →';find('step-position').textContent=step+' / 3';
    const selected=choice==='none'?'无预设':choice==='draft'?'当前设计':DS.presets.find(p=>'builtin:'+p.id===choice)?.name||presets.find(p=>'user:'+p.id===choice)?.name||'当前设计';
    find('selected-direction').textContent=selected;find('delete-preset').hidden=!appliedChoice.startsWith('user:');find('delete-preset').disabled=!writable;
  }
  function changeStep(next,focus=true){
    if(!loaded)return;
    if(step===1&&next!==1&&choice!==appliedChoice){
      const preset=choice==='none'?{config:emptyConfig}:choice==='draft'?{config}:DS.presets.find(p=>'builtin:'+p.id===choice)||presets.find(p=>'user:'+p.id===choice);
      if(!preset)return;config=DS.clone(preset.config);appliedChoice=choice;hasDraft=true;message('');scheduleSave();
    }
    dispose();dispose=()=>{};step=next;root.classList.remove('preview-maximized','editor-open');find('maximize-preview').textContent='放大预览';find('maximize-preview').setAttribute('aria-pressed','false');find('toggle-editor').setAttribute('aria-pressed','false');updateSteps();
    if(step===1)collection();if(step===2){render(false);if(choice===appliedChoice)find('preview-container').scrollTop=0;}if(step===3)updateOutput();
    if(focus)find('step-'+step+'-title').focus({preventScroll:true});
  }
  function editorTab(tab){
    for(const button of root.querySelectorAll('[data-editor-tab]')){const active=button.dataset.editorTab===tab;button.setAttribute('aria-selected',String(active));button.tabIndex=active?0:-1;}
    find('library-panel').hidden=tab!=='layout';find('inspector-panel').hidden=tab!=='parameters';find('editor-scroll').scrollTop=0;
  }
  function render(save){
    const focusLabel=document.activeElement?.getAttribute('aria-label');dispose();library();modules();controls();
    find('preview-container').classList.toggle('is-mobile',config.viewport==='mobile');find('preview-container').classList.toggle('is-tablet',config.viewport==='tablet');
    if(step===2)dispose=DS.render(find('studio-export'),config);else dispose=()=>{};
    find('preview-title').textContent=DS.lists.layout[config.layout];find('preview-summary').textContent=DS.lists.platform[config.platform]+' · '+DS.lists.appearance[config.appearance];updateOutput();updateSteps();
    if(focusLabel&&document.activeElement===document.body)[...root.querySelectorAll('[aria-label]')].find(element=>element.getAttribute('aria-label')===focusLabel)?.focus({preventScroll:true});if(save)scheduleSave();
  }
  root.querySelectorAll('[data-step]').forEach(button=>button.addEventListener('click',()=>changeStep(Number(button.dataset.step))));
  find('step-next').addEventListener('click',()=>changeStep(step+1));find('step-back').addEventListener('click',()=>changeStep(step-1));
  root.querySelectorAll('[data-editor-tab]').forEach(button=>button.addEventListener('click',()=>editorTab(button.dataset.editorTab)));
  find('toggle-editor').addEventListener('click',()=>{const visible=root.classList.toggle('editor-open');find('toggle-editor').setAttribute('aria-pressed',String(visible));find('toggle-editor').textContent=visible?'返回预览':'微调';});
  find('collection-filter').addEventListener('change',collection);
  find('studio-export').addEventListener('design-preview-status',event=>find('preview-info').textContent=event.detail);
  for(const [id,key]of [['project-title','title'],['project-purpose','purpose'],['target-platform','platform'],['canvas-mode','canvasMode'],['viewport','viewport'],['columns','columns'],['density','density'],['radius','radius'],['gap','gap'],['accent','accent'],['duration','duration'],['data-size','dataSize']]){
    const element=find(id),change=()=>{config[key]=['columns','radius','gap','duration','dataSize'].includes(key)?Number(element.value):element.value;if(key==='platform')config.viewport=config.platform==='android'?'mobile':'desktop';if((key==='title'||key==='purpose')&&!element.value.trim())return;const next=DS.normalize(config);config=next.config;message(next.notes.join(' '));render(true);};
    element.addEventListener(['project-title','project-purpose','radius','gap','accent'].includes(id)?'input':'change',change);
    if(id==='project-title'||id==='project-purpose')element.addEventListener('blur',()=>{if(!element.value.trim()){config[key]=emptyConfig[key];render(true);}});
  }
  find('dark-preview').addEventListener('change',()=>{config.dark=find('dark-preview').checked;render(true);});
  find('save-preset').addEventListener('click',async()=>{const name=find('preset-name').value.trim();if(!name){find('preset-name').focus();return;}if(!writable){message('无法保存，请先复制配置。',true);return;}if(presets.some(p=>p.name===name)){message('已有同名预设，请换一个名称。',true);return;}if(presets.length>=24){message('最多保存 24 个自定义预设。',true);return;}clearTimeout(autosave);const preset={id:crypto.randomUUID(),name,config:DS.validate(config)};presets.push(preset);if(await persist()){choice=appliedChoice='user:'+preset.id;updateSteps();message('预设已保存。');}});
  let pendingDelete='';find('delete-preset').addEventListener('click',async()=>{if(!appliedChoice.startsWith('user:'))return;const id=appliedChoice.slice(5);if(pendingDelete!==id){pendingDelete=id;message('再次点击删除此预设，当前设计会保留。');return;}clearTimeout(autosave);presets=presets.filter(p=>p.id!==id);if(await persist()){choice=appliedChoice='draft';hasDraft=true;pendingDelete='';updateSteps();message('已删除预设。');}});
  root.querySelectorAll('[data-output]').forEach(button=>button.addEventListener('click',()=>{output=button.dataset.output;updateOutput();}));
  const tabs=[...root.querySelectorAll('[data-output]')];tabs.forEach((tab,index)=>tab.addEventListener('keydown',event=>{const offset=event.key==='ArrowRight'?1:event.key==='ArrowLeft'?-1:0;if(offset){event.preventDefault();const next=tabs[(index+offset+tabs.length)%tabs.length];next.focus();next.click();}}));
  for(const group of ['[data-step]','[data-editor-tab]']){const buttons=[...root.querySelectorAll(group)];buttons.forEach((button,index)=>button.addEventListener('keydown',event=>{const offset=event.key==='ArrowRight'?1:event.key==='ArrowLeft'?-1:0;if(offset){event.preventDefault();const next=buttons[(index+offset+buttons.length)%buttons.length];next.focus();next.click();}}));}
  find('design-collection').addEventListener('keydown',event=>{const button=event.target.closest('[data-choice]');if(!button)return;const cards=[...find('design-collection').querySelectorAll('button')],offset=event.key==='ArrowRight'?1:event.key==='ArrowLeft'?-1:0;if(offset){event.preventDefault();cards[(cards.indexOf(button)+offset+cards.length)%cards.length].focus();}});
  find('copy-output').addEventListener('click',()=>{const text=find('output-text');text.focus();text.select();try{if(document.execCommand('copy')){message('已复制。');return;}}catch{}message('已全选，请按 Ctrl+C（macOS：Command+C）。');});
  find('maximize-preview').addEventListener('click',()=>{const full=root.classList.toggle('preview-maximized');find('maximize-preview').textContent=full?'返回微调':'放大预览';find('maximize-preview').setAttribute('aria-pressed',String(full));});
  find('replay-motion').addEventListener('click',()=>{dispose();dispose=DS.render(find('studio-export'),config);});
  let previousFocus;const closeImport=()=>{find('import-panel').hidden=true;previousFocus?.focus();};
  find('import-config').addEventListener('click',()=>{previousFocus=document.activeElement;find('import-panel').hidden=false;find('import-error').textContent='';find('import-text').focus();});find('cancel-import').addEventListener('click',closeImport);
  find('confirm-import').addEventListener('click',()=>{try{const text=find('import-text').value;if(new TextEncoder().encode(text).length>32768)throw Error('配置最多 32 KiB');config=DS.validate(JSON.parse(text));choice=appliedChoice='draft';hasDraft=true;closeImport();changeStep(2);render(true);message('');}catch(error){find('import-error').textContent=String(error.message||error);}});
  find('import-panel').addEventListener('keydown',event=>{if(event.key==='Escape'){event.preventDefault();closeImport();}if(event.key==='Tab'){const focusable=[...find('import-panel').querySelectorAll('textarea,button')],first=focusable[0],last=focusable.at(-1);if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}}});
  editorTab('layout');updateSteps();collection();updateOutput();
  (async()=>{try{if(!globalThis.toolhub?.storage)throw Error('当前宿主缺少扩展预设存储');const stored=await toolhub.storage.get();revision=stored.revision;if(stored.data!==null){const data=stored.data;if(data.schema!=='toolhub.design-presets/v1'||!Array.isArray(data.presets)||data.presets.length>24||Object.keys(data).some(key=>!['schema','presets','last'].includes(key)))throw Error('本机预设格式异常，原数据未覆盖');const ids=new Set();presets=data.presets.map(preset=>{if(!preset||Object.keys(preset).some(key=>!['id','name','config'].includes(key))||typeof preset.id!=='string'||preset.id.length>64||ids.has(preset.id)||typeof preset.name!=='string'||!preset.name.trim()||preset.name.length>40)throw Error('本机预设条目无效，原数据未覆盖');ids.add(preset.id);return {...preset,config:DS.validate(preset.config)};});config=DS.validate(data.last);hasDraft=true;choice=appliedChoice='draft';}loaded=true;writable=true;collection();updateSteps();updateOutput();find('storage-status').textContent='本机预设已就绪';}catch(error){loaded=true;writable=false;collection();updateSteps();find('storage-status').textContent='仅本次会话';message(String(error.message||error),true);}})();
})();
