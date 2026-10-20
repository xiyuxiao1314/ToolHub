(() => {
  'use strict';
  const lists = {
    layout: {grid:'卡片网格',metro:'Metro 磁贴',masonry:'瀑布流',list:'主从列表',canvas:'无限画布',kanban:'任务看板',spotlight:'全屏叙事',filmstrip:'横向画册',index:'瑞士目录',bento:'产品 Bento'},
    navigation: {sidebar:'侧边导航',rail:'紧凑图标栏',top:'顶部导航',none:'无导航'},
    contentStyle: {covers:'封面叙事',typographic:'字体海报',projects:'项目摘要'},
    canvasMode: {mosaic:'大小磁贴拼贴',scatter:'散点灵感板'},
    appearance: {immersive:'沉浸式画廊',clean:'简洁工作台',glass:'轻透 Fluent',editorial:'杂志排版',brutalist:'硬边设计'},
    effects: {expand:'卡片展开',hover:'封面悬停',stagger:'入场错峰',drag:'拖拽排序',sticky:'搜索栏吸顶',focus:'焦点强调'},
    performance: {standard:'标准渲染',virtual:'虚拟列表',culling:'画布视口裁剪'},
    modules: {hero:'标题与简介',search:'内容搜索',filters:'分类筛选',content:'主要内容',footer:'页脚状态'},
    platform: {web:'网页',android:'Android',desktop:'桌面应用'}
  };
  const defaultConfig = {schema:'toolhub.design/v2',contentStyle:'covers',canvasMode:'mosaic',title:'MUSE / 灵感档案',purpose:'作品、灵感与未完成的想法。',platform:'web',viewport:'desktop',layout:'metro',navigation:'top',appearance:'immersive',effects:['expand','hover','focus'],performance:'standard',modules:['hero','search','filters','content','footer'],columns:3,radius:2,gap:6,density:'comfortable',accent:'#d4ef9e',duration:360,dataSize:12,dark:true};
  const clone = value => JSON.parse(JSON.stringify(value));
  function validate(value) {
    if(!value || typeof value!=='object' || Array.isArray(value))throw Error('配置必须为 JSON 对象');
    if(value.schema==='toolhub.design/v1'){
      const legacyKeys=Object.keys(defaultConfig).filter(key=>!['contentStyle','canvasMode'].includes(key));
      if(Object.keys(value).length!==legacyKeys.length||Object.keys(value).some(key=>!legacyKeys.includes(key)))throw Error('旧版配置字段无效');
      value={...value,schema:'toolhub.design/v2',contentStyle:'projects',canvasMode:'scatter'};
    }
    if(Object.keys(value).some(key=>!Object.hasOwn(defaultConfig,key)) || Object.keys(value).length!==Object.keys(defaultConfig).length)throw Error('配置字段不完整或包含未知字段');
    const c=clone(value);
    if(c.schema!=='toolhub.design/v2')throw Error('仅支持 toolhub.design/v1、v2');
    for(const key of ['title','purpose'])if(typeof c[key]!=='string'||c[key].length>(key==='title'?48:160)||!c[key].trim()||/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(c[key]))throw Error('名称与用途须为有效文本');
    for(const key of ['layout','navigation','appearance','performance','platform','contentStyle','canvasMode'])if(typeof c[key]!=='string'||!Object.hasOwn(lists[key],c[key]))throw Error('未知选项：'+key);
    if(!['desktop','tablet','mobile'].includes(c.viewport)||!['comfortable','compact'].includes(c.density)||typeof c.dark!=='boolean')throw Error('画幅、密度或主题无效');
    for(const key of ['effects','modules'])if(!Array.isArray(c[key])||c[key].length>Object.keys(lists[key]).length||new Set(c[key]).size!==c[key].length||c[key].some(item=>typeof item!=='string'||!Object.hasOwn(lists[key],item)))throw Error('效果或模块无效');
    if(!c.modules.includes('content'))throw Error('必须保留主要内容模块');
    if(!Number.isInteger(c.radius)||c.radius<0||c.radius>24||!Number.isInteger(c.gap)||c.gap<2||c.gap>28||![2,3,4].includes(c.columns)||![0,160,240,360,480].includes(c.duration)||![12,100,10000].includes(c.dataSize)||typeof c.accent!=='string'||!/^#[0-9a-f]{6}$/i.test(c.accent))throw Error('设计参数超出支持范围');
    if(c.performance==='virtual'&&c.layout!=='list')throw Error('第一版虚拟渲染适用于主从列表；其他结构使用标准渲染');
    if(c.performance==='culling'&&c.layout!=='canvas')throw Error('视口裁剪适用于无限画布');
    if(c.dataSize===10000&&c.performance!=='virtual')throw Error('一万条示例仅在虚拟列表模式开放');
    if(c.effects.includes('drag')&&(c.performance==='virtual'||c.layout==='canvas'))throw Error('虚拟列表 / 无限画布不启用卡片拖拽排序；画布保留平移');
    return c;
  }
  function normalize(value) {
    const c=clone(value),notes=[];
    if(c.performance==='virtual'&&c.layout!=='list'){c.performance=c.layout==='canvas'?'culling':'standard';notes.push('此结构已改用'+lists.performance[c.performance]+'；虚拟列表需要主从列表。');}
    if(c.performance==='culling'&&c.layout!=='canvas'){c.performance='standard';notes.push('视口裁剪只用于画布，已切回标准渲染。');}
    if(c.dataSize===10000&&c.performance!=='virtual'){c.dataSize=100;notes.push('当前结构使用 100 条示例，避免一次渲染一万条内容。');}
    if(c.effects.includes('drag')&&(c.performance==='virtual'||c.layout==='canvas')){c.effects=c.effects.filter(e=>e!=='drag');notes.push('已关闭此组合的卡片拖拽排序，保留滚动 / 画布平移。');}
    return {config:validate(c),notes};
  }
  function escapeHtml(value) { return String(value).replace(/[&<>"']/g,character=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[character])); }
  // Original vector studies. No third-party artwork, remote fetches or dependencies.
  function artwork(index) {
    const scenes=[
      ['#d9c4a7','#aa5336','<circle cx="285" cy="114" r="56" fill="#f4c87d"/><path d="M0 250L100 185 201 251 310 186 400 241V400H0Z" fill="#766e61"/><path d="M88 400V173a112 112 0 01224 0v227h-54V173a58 58 0 00-116 0v227Z" fill="#d7b69b"/><path d="M258 400V173a58 58 0 00-58-58v285" fill="#725644"/><path d="M0 353l142-46v93H0" fill="#e8d7b6"/>'],
      ['#283b42','#d6c6a5','<circle cx="95" cy="93" r="43" fill="#decda2"/><path d="M0 250l82-90 54 55 92-119 83 126 89-59v237H0Z" fill="#53675f"/><path d="M0 315l139-119 114 113 75-47 72 46v92H0Z" fill="#253b39"/><path d="M178 400l51-93-5-41 49 30-23 38 37 66" fill="#a7b5a0"/>'],
      ['#eadcbd','#a94a31','<circle cx="270" cy="150" r="102" fill="#ba4d31"/><path d="M0 267c85-97 139 67 230-2s118-61 170-9v144H0" fill="#243d46"/><path d="M0 310c98-74 120 66 224 0s118-45 176-11" stroke="#d2ae78" stroke-width="2" fill="none"/><path d="M0 325c98-74 120 66 224 0s118-45 176-11" stroke="#d2ae78" stroke-width="2" fill="none"/>'],
      ['#323349','#d9a9a4','<ellipse cx="200" cy="217" rx="122" ry="160" fill="#98728c"/><ellipse cx="200" cy="218" rx="88" ry="129" fill="#d1a6ae"/><ellipse cx="200" cy="218" rx="55" ry="94" fill="#654e70"/><ellipse cx="200" cy="218" rx="25" ry="60" fill="#f0cbc0"/><path d="M0 335l400-153v60L0 395Z" fill="#343247"/>'],
      ['#d7dacb','#2e564d','<path d="M200 400V43" stroke="#4a6651" stroke-width="4"/><path d="M197 290C60 292 54 175 71 133c87 8 140 62 126 157M202 235C346 225 351 116 331 85c-72 0-142 68-129 150M198 167C123 165 103 76 119 47c65 4 93 73 79 120" fill="#53735e"/><path d="M90 155l103 123M310 112L207 221M132 69l64 85" stroke="#aab397" stroke-width="2"/>'],
      ['#de7442','#f1cf92','<path d="M40 400V95h90v305M145 400V30h100v370M260 400V160h100v240" fill="#ead2ad"/><path d="M65 400V120h45v280M170 400V55h50v345M285 400V185h50v215" fill="#6d5e51"/><path d="M0 345l400-131v30L0 375" fill="#bd542c"/>'],
      ['#a7b9bd','#e7e1d2','<circle cx="205" cy="173" r="115" fill="#dae0d6"/><path d="M30 285l166-164 171 164Z" fill="#6b7f83"/><path d="M83 285l113-112 117 112Z" fill="#b2bcb4"/><path d="M196 121v164h171" fill="#485d65"/><path d="M0 310h400M0 340h400M0 370h400" stroke="#e0e5dd" stroke-width="2"/>'],
      ['#e6bdb4','#5e5c84','<circle cx="208" cy="193" r="115" fill="#bd6c61"/><ellipse cx="208" cy="193" rx="62" ry="115" fill="#f2d7c0"/><ellipse cx="208" cy="193" rx="20" ry="115" fill="#61617e"/><path d="M0 310l400-60v150H0" fill="#7a7a8f"/><path d="M35 369l345-49" stroke="#e5c9ba" stroke-width="3"/>']
    ];
    const [bg,ink,shape]=scenes[index%scenes.length];
    return `<svg class="p-art" viewBox="0 0 400 400" preserveAspectRatio="xMidYMid slice" aria-hidden="true"><rect width="400" height="400" fill="${bg}"/>${shape}<g fill="none" stroke="${ink}" opacity=".2"><path d="M18 18h364v364H18Z"/><path d="M24 24h352v352H24Z"/></g><text x="31" y="48" fill="${ink}" font-family="Georgia,serif" font-size="12" letter-spacing="4">MUSE / STUDIES</text><text x="365" y="373" fill="${ink}" text-anchor="end" font-family="Georgia,serif" font-size="13">${String(index+1).padStart(2,'0')}</text></svg>`;
  }
  function sample(index) {
    const names=['静谧建筑','山间来信','日落之后','柔软的回声','植物研究所','光的秩序','浮岛档案','周日漫游','品牌官网','设计参考集','移动端原型','组件文档'];
    return {id:'project-'+index,name:names[index%12]+(index>=12?' '+(index+1):''),subtitle:['QUIET ARCHITECTURE','LETTERS FROM THE HILLS','AFTER THE SUN','SOFT ECHOES','BOTANICAL STUDIES','THE ORDER OF LIGHT','FLOATING ARCHIVE','SUNDAY WANDER'][index%8],type:['design','development','assets'][index%3],label:['设计','开发','素材'][index%3],index};
  }
  function card(item,c) {
    return `<article class="p-item p-study-${item.index%8}" data-id="${item.id}" data-index="${item.index}" data-type="${item.type}"${c.effects.includes('drag')?' draggable="true"':''}><button type="button" class="p-item-main" aria-expanded="false"${c.effects.includes('drag')?' draggable="true"':''}>${c.contentStyle==='covers'?artwork(item.index):''}${c.contentStyle==='typographic'?'<span class="p-poster-word" aria-hidden="true">'+['FORM','野生','MAKE','遊','IDEA','空間'][item.index%6]+'</span>':''}<span class="p-item-tag">${item.label} / ${String(item.index+1).padStart(2,'0')}</span><span class="p-item-copy"><small class="p-item-subtitle">${item.subtitle}</small><h3>${escapeHtml(item.name)}</h3><small class="p-item-hint">${c.effects.includes('expand')?'探索作品 ↗':'查看项目 ↗'}</small></span></button><div class="p-item-more" hidden>${item.subtitle} / ${item.label}</div></article>`;
  }
  function canvasPosition(index,c) {
    const unit=150,gap=c.gap;
    if(c.canvasMode==='scatter')return {x:(index%6)*235+(Math.floor(index/6)%2)*65,y:Math.floor(index/6)*250+(index%3)*24,w:210,h:225};
    const tiles=[[0,0,2,2],[2,0,1,1],[3,0,1,2],[4,0,2,1],[2,1,1,1],[4,1,1,1],[5,1,1,2],[0,2,1,1],[1,2,2,1],[3,2,2,1]];
    const [x,y,w,h]=tiles[index%10],group=Math.floor(index/10);
    return {x:(x+(group%3)*6)*unit,y:(y+Math.floor(group/3)*3)*unit,w:w*unit-gap,h:h*unit-gap};
  }
  function installStage(root,c,signal) {
    let layer=null,origin=null,phase='closed',generation=0,flights=[];
    const surface=root.closest('#design-studio')||root,wasInert=surface.inert;
    const motion=()=>!matchMedia('(prefers-reduced-motion: reduce)').matches&&c.duration>0;
    const stop=()=>{for(const item of flights){item.animation.cancel();item.node.remove();}flights=[];};
    const restore=()=>{generation++;stop();layer?.remove();layer=null;phase='closed';surface.inert=wasInert;origin?.querySelector('button')?.setAttribute('aria-expanded','false');origin?.querySelector('button')?.focus({preventScroll:true});};
    function fly(art,from,to){
      const node=document.createElement('div');node.className='p-flight';node.innerHTML=art.innerHTML;Object.assign(node.style,{left:to.x+'px',top:to.y+'px',width:to.width+'px',height:to.height+'px',borderRadius:c.radius+'px'});layer.append(node);
      const transform=rect=>`translate(${rect.x-to.x}px,${rect.y-to.y}px) scale(${rect.width/to.width},${rect.height/to.height})`;
      const animation=node.animate([{transform:transform(from)},{transform:transform(to)}],{duration:c.duration,easing:'cubic-bezier(.2,.8,.2,1)',fill:'both'});flights.push({node,animation});return animation.finished.catch(()=>{});
    }
    async function close(){
      if(!layer||phase==='closing')return;
      if(phase==='opening'||!motion()||!origin?.isConnected){restore();return;}
      phase='closing';const id=++generation,panel=layer.querySelector('.p-expanded'),art=panel.querySelector('.p-expanded-art'),from=art.getBoundingClientRect(),to=origin.getBoundingClientRect();
      if(to.bottom<0||to.top>innerHeight||to.right<0||to.left>innerWidth){restore();return;}
      const finished=fly(art,from,to);panel.style.visibility='hidden';await finished;if(id===generation)restore();
    }
    const open=async element=>{
      if(layer)return;origin=element;phase='opening';const id=++generation,item=sample(Number(element.dataset.index)),from=element.getBoundingClientRect();element.querySelector('button').setAttribute('aria-expanded','true');
      layer=document.createElement('div');layer.className='ds p-modal-host p-'+c.appearance+(c.dark?' p-dark':'');for(const key of ['--p-accent','--p-radius','--p-ms'])layer.style.setProperty(key,getComputedStyle(root).getPropertyValue(key));
      layer.innerHTML=`<div class="p-stage-layer"><section class="p-expanded" role="dialog" aria-modal="true" aria-label="${escapeHtml(item.name)} · 作品详情" tabindex="-1"><button type="button" class="p-close" aria-label="关闭作品详情">×</button><div class="p-expanded-art">${c.contentStyle==='covers'?artwork(item.index):'<span class="p-expanded-type">'+escapeHtml(item.subtitle.split(' ')[0])+'</span>'}</div><div class="p-expanded-copy"><small>SELECTED WORK / ${String(item.index+1).padStart(2,'0')}</small><h2>${escapeHtml(item.name)}</h2><p>${item.subtitle}</p><span>${item.label} / MUSE STUDIES</span><button type="button" class="p-back">返回作品墙 ↙</button></div></section></div>`;
      document.body.append(layer);surface.inert=true;const panel=layer.querySelector('.p-expanded'),art=panel.querySelector('.p-expanded-art'),copy=layer.querySelector('.p-expanded-copy');panel.focus({preventScroll:true});
      if(motion()){art.style.opacity='0';copy.style.opacity='0';await fly(art,from,art.getBoundingClientRect());if(id!==generation)return;stop();art.style.opacity='1';copy.style.opacity='1';}
      if(id===generation)phase='open';
    };
    document.addEventListener('click',event=>{if(layer?.contains(event.target)&&(event.target.closest('.p-close,.p-back')||event.target.classList.contains('p-stage-layer')))close();},{signal});
    document.addEventListener('keydown',event=>{if(!layer)return;if(event.key==='Escape'){event.preventDefault();close();}if(event.key==='Tab'){const buttons=[...layer.querySelectorAll('button')],first=buttons[0],last=buttons.at(-1);if(event.shiftKey&&(document.activeElement===first||document.activeElement===layer.querySelector('.p-expanded'))){event.preventDefault();last.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}}},{signal});
    open.dispose=()=>{generation++;stop();if(layer)surface.inert=wasInert;layer?.remove();layer=null;phase='closed';};return open;
  }
  function markup(c) {
    const nav=`<nav class="p-nav" aria-label="示例应用导航"><div class="p-wordmark">${c.navigation==='rail'?'◧':escapeHtml(c.title)}</div>${['项目','收藏','归档'].map((label,i)=>`<button type="button" data-nav="${label}" class="${i===0?'is-active':''}" aria-pressed="${i===0}">${label}</button>`).join('')}</nav>`;
    const cards=Array.from({length:c.dataSize===10000?12:c.dataSize},(_,i)=>card(sample(i),c));
    let content=`<div class="p-items">${cards.join('')}</div>`;
    if(c.layout==='spotlight'||c.layout==='filmstrip')content='<div class="p-filmstrip" tabindex="0" aria-label="横向作品画册">'+cards.join('')+'</div>';
    if(c.layout==='list')content=(c.performance==='virtual'?'<div><div class="p-virtual-count" role="status"></div><div class="p-virtual" aria-label="虚拟项目列表"><div class="p-spacer"></div></div></div>':content)+'<aside class="p-detail"><h3>选择一个项目</h3></aside>';
    if(c.layout==='canvas')content='<div class="p-canvas" aria-label="项目画布" tabindex="0"><div class="p-world"></div><div class="p-zoom"><button type="button" data-zoom="out" aria-label="缩小画布">−</button><output>80%</output><button type="button" data-zoom="in" aria-label="放大画布">＋</button><button type="button" data-zoom="reset">复位</button></div></div><div class="p-virtual-count" role="status"></div>';
    if(c.layout==='kanban')content='<div class="p-kanban">'+['想法','进行中','已完成'].map((name,lane)=>`<section class="p-lane" data-lane="${lane}"><h3>${name}</h3>${cards.filter((_,i)=>i%3===lane).join('')}</section>`).join('')+'</div>';
    const modules={hero:`<header class="p-hero"><small>COLLECTED IDEAS / VOL. 01</small><h2>${escapeHtml(c.title)}</h2><span class="p-hero-index" aria-hidden="true">01—12</span><p>${escapeHtml(c.purpose)}</p></header>`,search:'<div class="p-search-module"><input class="p-search" aria-label="搜索示例项目" placeholder="搜索项目名称…"/></div>',filters:'<div class="p-filters" aria-label="示例内容分类"><button type="button" data-filter="all" class="is-active">全部</button><button type="button" data-filter="design">设计</button><button type="button" data-filter="development">开发</button><button type="button" data-filter="assets">素材</button></div>',content:`<section class="p-content" aria-label="项目内容">${content}</section>`,footer:`<footer class="p-footer"><span>本机示例 · ${c.dataSize.toLocaleString()} 条项目</span><output class="p-result" role="status">等待选择</output></footer>`};
    const chrome=c.platform==='android'?'<div class="p-chrome p-status"><b>9:41</b><span>ANDROID · ADAPTIVE</span><span aria-hidden="true">◒ ▰</span></div>':c.platform==='desktop'?'<div class="p-chrome p-window"><span aria-hidden="true">● ● ●</span><span>WORKSPACE / DESKTOP</span><span aria-hidden="true">− □ ×</span></div>':'<div class="p-chrome p-browser"><span aria-hidden="true">← → ↻</span><span>◈ &nbsp; muse.local / collection</span><span>WEB</span></div>';
    return `${chrome}<div class="p-app p-${c.navigation}">${nav}<main class="p-main">${c.modules.map(id=>modules[id]).join('')}</main></div>${c.platform==='android'?'<div class="p-gesture" aria-hidden="true"><span></span></div>':''}`;
  }
  function bind(root,c) {
    let expanded=new Set(),query='',filter='all',dragged=null,dispose=()=>{};
    const controller=new AbortController(),listenerOptions={signal:controller.signal};
    const result=text=>{const element=root.querySelector('.p-result');if(element)element.textContent=text;root.dispatchEvent(new CustomEvent('design-preview-status',{detail:text,bubbles:true}));};
    const itemMatches=item=>(filter==='all'||item.type===filter)&&item.name.toLowerCase().includes(query);
    const visible=()=>Array.from({length:c.dataSize},(_,i)=>i).filter(i=>itemMatches(sample(i)));
    let indices=visible(),rerender=()=>{};
    const closeStage=installStage(root,c,controller.signal);
    const select=(element)=>{if(c.effects.includes('expand')&&c.layout!=='list'){closeStage(element);return;}const index=Number(element.dataset.index),item=sample(index);if(c.effects.includes('expand')){expanded.has(index)?expanded.delete(index):expanded.add(index);const open=expanded.has(index);element.classList.toggle('is-open',open);element.querySelector('button').setAttribute('aria-expanded',String(open));element.querySelector('.p-item-more').hidden=!open;}
      const detail=root.querySelector('.p-detail');if(detail){detail.replaceChildren();const heading=document.createElement('h3');heading.textContent=item.name;const summary=document.createElement('p');summary.textContent=item.label+' / '+item.subtitle;const next=document.createElement('p');next.textContent='MUSE STUDIES / '+String(index+1).padStart(2,'0');detail.append(heading,summary,next);}result('已选择：'+item.name);rerender();};
    root.addEventListener('click',event=>{const button=event.target.closest('button');if(!button)return;if(button.matches('[data-nav]')){root.querySelectorAll('[data-nav]').forEach(item=>{const active=item===button;item.classList.toggle('is-active',active);item.setAttribute('aria-pressed',String(active));});result('导航状态：'+button.dataset.nav+'（示例）');}if(button.classList.contains('p-item-main'))select(button.closest('.p-item'));if(button.matches('[data-filter]')){filter=button.dataset.filter;root.querySelectorAll('[data-filter]').forEach(item=>item.classList.toggle('is-active',item===button));applyFilter();}},listenerOptions);
    function applyFilter(){indices=visible();if(c.performance==='virtual'){const scroll=root.querySelector('.p-virtual');scroll.scrollTop=0;rerender();}else if(c.layout==='canvas'){rerender();}else{root.querySelectorAll('.p-item').forEach(element=>{element.hidden=!itemMatches(sample(Number(element.dataset.index)));});}result('匹配 '+indices.length+' 个项目');}
    root.querySelector('.p-search')?.addEventListener('input',event=>{query=event.target.value.trim().toLowerCase();applyFilter();});
    if(c.effects.includes('drag')){
      root.addEventListener('dragstart',event=>{dragged=event.target.closest('.p-item');if(dragged){dragged.classList.add('is-dragging');event.dataTransfer.setData('text/plain',dragged.dataset.id);event.dataTransfer.effectAllowed='move';}},listenerOptions);
      root.addEventListener('dragover',event=>{if(dragged&&event.target.closest('.p-item,.p-lane'))event.preventDefault();},listenerOptions);
      root.addEventListener('drop',event=>{if(!dragged)return;event.preventDefault();const target=event.target.closest('.p-item'),lane=event.target.closest('.p-lane');if(target&&target!==dragged)target.before(dragged);else if(lane)lane.append(dragged);result('已在预览中调整项目顺序');},listenerOptions);
      root.addEventListener('dragend',()=>{dragged?.classList.remove('is-dragging');dragged=null;},listenerOptions);
    }
    if(c.performance==='virtual'){
      const scroll=root.querySelector('.p-virtual'),spacer=root.querySelector('.p-spacer'),counter=root.querySelector('.p-virtual-count');let nodes=new Map(),prefix=[],queued=false;
      const reduced=()=>matchMedia('(prefers-reduced-motion: reduce)').matches;
      rerender=()=>{
        prefix=[0];for(const index of indices)prefix.push(prefix[prefix.length-1]+(expanded.has(index)?164:86));spacer.style.height=prefix[prefix.length-1]+'px';
        let lo=0,hi=indices.length;while(lo<hi){const mid=(lo+hi)>>1;if(prefix[mid+1]<scroll.scrollTop)lo=mid+1;else hi=mid;}
        const start=Math.max(0,lo-2);let end=lo;while(end<indices.length&&prefix[end]<scroll.scrollTop+scroll.clientHeight)end++;end=Math.min(indices.length,end+2);
        const keep=new Set(indices.slice(start,end));for(const [index,node]of nodes)if(!keep.has(index)){node.remove();nodes.delete(index);}
        for(let position=start;position<end;position++){
          const index=indices[position];let node=nodes.get(index);if(!node){const template=document.createElement('template');template.innerHTML=card(sample(index),c);node=template.content.firstElementChild;nodes.set(index,node);spacer.append(node);}
          const old=node.getBoundingClientRect(),open=expanded.has(index);node.style.transform=`translateY(${prefix[position]}px)`;node.style.height=(open?154:76)+'px';node.classList.toggle('is-open',open);node.querySelector('button').setAttribute('aria-expanded',String(open));node.querySelector('.p-item-more').hidden=!open;
          if(!reduced()&&c.duration&&old.height>0&&node.isConnected){const target=node.getBoundingClientRect(),shift=old.top-target.top;if(Math.abs(shift)>1)node.animate([{translate:`0 ${shift}px`},{translate:'0 0'}],{duration:c.duration,easing:'ease-out'});}
        }
        counter.textContent=indices.length.toLocaleString()+' 条匹配 · 实际渲染 '+nodes.size+' 条';
      };
      scroll.addEventListener('scroll',()=>{if(queued)return;queued=true;requestAnimationFrame(()=>{queued=false;rerender();});},{passive:true});const observer=new ResizeObserver(()=>rerender());observer.observe(scroll);dispose=()=>observer.disconnect();rerender();
    }
    if(c.layout==='canvas'){
      const canvas=root.querySelector('.p-canvas'),world=root.querySelector('.p-world'),output=root.querySelector('.p-zoom output'),counter=root.querySelector('.p-virtual-count');let camera={x:8,y:8,scale:.8},drag=null;
      rerender=()=>{world.style.transform=`translate(${camera.x}px,${camera.y}px) scale(${camera.scale})`;world.replaceChildren();let rendered=0;
        indices.forEach(index=>{const {x,y,w,h}=canvasPosition(index,c);if(c.performance==='culling'&&(x+w<-camera.x/camera.scale||x>(canvas.clientWidth-camera.x)/camera.scale||y+h<-camera.y/camera.scale||y>(canvas.clientHeight-camera.y)/camera.scale))return;const template=document.createElement('template');template.innerHTML=card(sample(index),c);const element=template.content.firstElementChild;Object.assign(element.style,{left:x+'px',top:y+'px',width:w+'px',height:h+'px'});world.append(element);rendered++;});output.textContent=Math.round(camera.scale*100)+'%';counter.textContent=indices.length+' 个节点 · 实际渲染 '+rendered+' 个';};
      canvas.addEventListener('pointerdown',event=>{if(event.target.closest('button')||event.button!==0)return;drag={x:event.clientX,y:event.clientY,left:camera.x,top:camera.y};canvas.setPointerCapture(event.pointerId);});canvas.addEventListener('pointermove',event=>{if(drag){camera.x=drag.left+event.clientX-drag.x;camera.y=drag.top+event.clientY-drag.y;rerender();}});const stop=()=>{drag=null;};canvas.addEventListener('pointerup',stop);canvas.addEventListener('pointercancel',stop);
      root.querySelectorAll('[data-zoom]').forEach(button=>button.addEventListener('click',()=>{if(button.dataset.zoom==='reset')camera={x:8,y:8,scale:.8};else{const next=Math.max(.3,Math.min(1.5,camera.scale+(button.dataset.zoom==='in'?.1:-.1))),x=canvas.clientWidth/2,y=canvas.clientHeight/2;camera.x=x-(x-camera.x)*next/camera.scale;camera.y=y-(y-camera.y)*next/camera.scale;camera.scale=next;}rerender();}));canvas.addEventListener('keydown',event=>{if(event.target!==canvas&&!event.target.closest('.p-zoom'))return;const direction={ArrowLeft:[30,0],ArrowRight:[-30,0],ArrowUp:[0,30],ArrowDown:[0,-30]}[event.key];if(direction){event.preventDefault();camera.x+=direction[0];camera.y+=direction[1];rerender();}});const observer=new ResizeObserver(rerender);observer.observe(canvas);dispose=()=>observer.disconnect();rerender();
    }
    root.querySelectorAll('.p-item').forEach((node,i)=>node.style.setProperty('--p-delay',Math.min(i,8)*35+'ms'));
    return ()=>{controller.abort();dispose();closeStage.dispose();};
  }
  function classes(c){return ['ds','p-platform-'+c.platform,'p-viewport-'+c.viewport,'p-content-'+c.contentStyle,'p-'+c.layout,'p-cols-'+c.columns,'p-'+c.appearance,...c.effects.map(e=>'p-'+e),c.dark?'p-dark':'',c.density==='compact'?'p-compact':'',c.layout==='canvas'?'p-canvas-layout':''].filter(Boolean).join(' ');}
  function tokens(c){return {'--p-accent':c.accent,'--p-gap':c.gap+'px','--p-radius':c.radius+'px','--p-ms':c.duration+'ms'};}
  function render(root,c){root.className=classes(c);Object.entries(tokens(c)).forEach(([key,value])=>root.style.setProperty(key,value));root.innerHTML=markup(c);return bind(root,c);}
  function exportHtml(c,css){const configuration=JSON.stringify(c).replace(/</g,'\\u003c').replace(/\u2028/g,'\\u2028').replace(/\u2029/g,'\\u2029');const runtime=`const artwork=${artwork.toString()};const canvasPosition=${canvasPosition.toString()};const installStage=${installStage.toString()};const escapeHtml=${escapeHtml.toString()};const sample=${sample.toString()};const card=${card.toString()};(${bind.toString()})(document.getElementById('studio-export'),${configuration});`;return '<!doctype html>\n<html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>'+escapeHtml(c.title)+'</title><style>body{margin:0;padding:24px;background:#eef2f8}'+css+'#studio-export{'+Object.entries(tokens(c)).map(([key,value])=>key+':'+value).join(';')+'}</style></head><body><div id="studio-export" class="'+classes(c)+'">'+markup(c)+'</div><script>'+runtime+'<\/script></body></html>';}
  function agentPrompt(c){const stack=c.platform==='android'?'采用项目现有 Android 技术栈；Compose 可使用原生自适应容器与 Lazy 列表。不要把 HTML 示例当作原生 Android 源码。':c.platform==='desktop'?'采用项目现有桌面框架，兼顾窗口缩放、键盘快捷键和系统行为。':'采用项目现有 Web 技术栈，保留语义化 HTML 与可访问的焦点顺序。';return `请根据以下已确认的设计组合，为「${c.title}」设计并实现${lists.platform[c.platform]}界面。\n\n用途：${c.purpose}\n内容语言：${lists.contentStyle[c.contentStyle]}；画布编排：${lists.canvasMode[c.canvasMode]}。\n页面结构：${lists.layout[c.layout]}；导航：${lists.navigation[c.navigation]}；视觉：${lists.appearance[c.appearance]}。\n模块顺序：${c.modules.map(id=>lists.modules[id]).join(' → ')}。\n交互效果：${c.effects.length?c.effects.map(id=>lists.effects[id]).join('、'):'无额外动效'}。\n性能策略：${lists.performance[c.performance]}；示例数据量 ${c.dataSize}。\n参数：${c.columns} 列、圆角 ${c.radius}px、间距 ${c.gap}px、${c.density==='compact'?'紧凑':'宽松'}密度、强调色 ${c.accent}、${c.dark?'深色':'浅色'}主题、动效 ${c.duration}ms。\n\n${stack}\n\n先理解真实业务、已有代码与内容，再使用此组合。视觉要求：保持封面主体、留白、字号层级和可读对比。磁贴采用有节奏的大小比例，展开保留来源位置并支持 Escape 返回；减少动效时直接切换。平台预览包含相应外壳和重排，仅为交互参考。HTML 是可运行的交互参考，不是完整业务成品，也不替代后端与数据模型。${c.performance==='virtual'?'虚拟列表必须只挂载视口与缓冲区，使用稳定 ID、动态高度测量，展开状态独立存储；滚动离屏后返回仍保留展开。':''}${c.layout==='canvas'?'区分世界坐标与屏幕坐标，支持平移、缩放、复位和可访问替代操作。':''}\n验收：手机/平板/桌面重排，长文本、空/加载/错误状态，键盘与触控，prefers-reduced-motion，滚动位置和选中状态；不要为效果擅自升级依赖或更换技术栈。\n\n配置：\n${JSON.stringify(c,null,2)}`;}
  const presets=[
    ['metro','01 / 沉浸封面墙',{layout:'metro',navigation:'top',modules:['hero','content','footer'],effects:['expand','hover','stagger','focus'],columns:4}],
    ['canvas','02 / 无界拼贴画布',{layout:'canvas',navigation:'rail',modules:['hero','content','footer'],performance:'culling',dataSize:100,effects:['expand','focus']}],
    ['gallery','03 / 纸上展览',{title:'Objects & Silence',layout:'masonry',navigation:'top',appearance:'editorial',dark:false,accent:'#943f26',radius:0,gap:24,effects:['expand','hover']}],
    ['portfolio','04 / 字体实验室',{title:'FORM / 不拘一格',layout:'metro',navigation:'none',appearance:'brutalist',contentStyle:'typographic',columns:3,dark:false,radius:0,gap:8,accent:'#526813',effects:['expand','stagger']}],
    ['creator','05 / 精密项目台',{title:'Studio / 进行之中',layout:'grid',navigation:'sidebar',appearance:'clean',contentStyle:'projects',dark:false,radius:8,gap:16,accent:'#246958',effects:['expand','hover','focus']}],
    ['library','06 / 万条素材目录',{title:'Archive / 素材索引',layout:'list',navigation:'sidebar',appearance:'clean',dark:false,accent:'#246958',radius:6,gap:12,performance:'virtual',dataSize:10000,effects:['expand','focus','sticky']}],
    ['kanban','07 / 流光工作流',{title:'Flow / 创意进行时',layout:'kanban',navigation:'top',appearance:'glass',contentStyle:'projects',dark:false,accent:'#236777',radius:14,gap:16,effects:['expand','drag','hover']}],
    ['mobile','08 / 掌中灵感馆',{title:'日常 / DAILY MUSE',layout:'grid',navigation:'top',platform:'android',viewport:'mobile',appearance:'editorial',dark:false,accent:'#92543e',radius:16,gap:10,columns:2,effects:['expand','focus']}]
    ,['cinema','09 / 电影感叙事',{title:'Beyond / 光影之外',purpose:'让作品占据视线，以一幕一幕的节奏，讲述你的故事。',layout:'spotlight',navigation:'none',modules:['hero','content','footer'],effects:['expand','focus'],radius:0,gap:2}]
    ,['swiss','10 / 瑞士设计目录',{title:'Selected / 作品索引',layout:'index',navigation:'top',appearance:'editorial',contentStyle:'covers',dark:false,accent:'#a83e28',radius:0,gap:2,effects:['expand','focus']}]
    ,['photo','11 / 横向摄影札记',{title:'Between Places',purpose:'沿着一条时间线，收集远行中安静而微小的瞬间。',layout:'filmstrip',navigation:'none',appearance:'editorial',dark:false,accent:'#7d563d',radius:2,gap:20,modules:['hero','content','footer'],effects:['expand','hover']}]
    ,['product','12 / 产品能力拼图',{title:'Make room for ideas.',purpose:'把你的工具、项目与灵感，安放在同一个创作空间。',layout:'bento',navigation:'top',appearance:'clean',contentStyle:'projects',dark:false,accent:'#366842',radius:20,gap:12,effects:['expand','hover'],columns:3}]
  ].map(([id,name,overrides])=>({id,name,config:validate({...clone(defaultConfig),...overrides})}));
  globalThis.DesignStudio=Object.freeze({lists,defaultConfig,presets,clone,validate,normalize,render,exportHtml,agentPrompt});
})();
