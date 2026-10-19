(()=>{
  const token=__TOKEN__;let seq=0;const pending=new Map();
  window.addEventListener('message',e=>{
    if(e.source!==parent||e.data?.channel!=='toolhub.sdk'||e.data.token!==token)return;
    if(e.data.type==='theme'){for(const [key,value]of Object.entries(e.data.colors??{})){if(typeof value==='string'&&value.length<=128)document.documentElement.style.setProperty('--'+key,value)}return}
    const p=pending.get(e.data.id);if(p){pending.delete(e.data.id);clearTimeout(p.timer);e.data.error?p.reject(new Error(e.data.error)):p.resolve(e.data.result)}
  });
  const send=(type,data)=>{const id=++seq;return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{pending.delete(id);reject(new Error('ToolHub request timeout'))},20000);pending.set(id,{resolve,reject,timer});parent.postMessage({channel:'toolhub.sdk',token,id,type,...data},'*')})};
  globalThis.toolhub=Object.freeze({call:(method,params={})=>send('call',{method,params}),navigate:id=>send('navigate',{page:id}),notify:message=>send('notify',{message}),storage:Object.freeze({get:()=>send('storage',{operation:'get'}),set:(data,revision)=>send('storage',{operation:'set',data,revision})})});
})();
