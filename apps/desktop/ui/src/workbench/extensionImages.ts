/** Image-only package storage. The opaque child never receives a disk path or DB handle. */
export const imageLimits={count:50,packageBytes:32*1024*1024,totalBytes:128*1024*1024,displayBytes:768*1024,wallBytes:128*1024,displaySide:1920,wallSide:768} as const
type Collection={packageId:string;revision:number;count:number;bytes:number}
type Metadata={packageId:string;id:string;name:string;width:number;height:number;wallWidth:number;wallHeight:number;bytes:number;createdAt:number}
type Pixels={packageId:string;id:string;display:Uint8Array;wall:Uint8Array}
let opening:Promise<IDBDatabase>|undefined
function database(){
  if(!opening)opening=new Promise<IDBDatabase>((resolve,reject)=>{
    const request=indexedDB.open('toolhub.extension-images.v1',1)
    request.onupgradeneeded=()=>{const db=request.result;db.createObjectStore('collections',{keyPath:'packageId'});db.createObjectStore('pixels',{keyPath:['packageId','id']});db.createObjectStore('metadata',{keyPath:['packageId','id']}).createIndex('package','packageId')}
    request.onsuccess=()=>{const db=request.result;db.onversionchange=()=>{db.close();opening=undefined};resolve(db)}
    request.onerror=()=>{opening=undefined;reject(Error('图片存储不可用：'+request.error?.name))}
    request.onblocked=()=>{opening=undefined;reject(Error('图片存储被其他窗口占用，请关闭旧窗口后重试'))}
  })
  return opening
}
const result=<T>(request:IDBRequest<T>)=>new Promise<T>((resolve,reject)=>{request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error??Error('图片存储请求失败'))})
function completion(tx:IDBTransaction){const done=new Promise<void>((resolve,reject)=>{tx.oncomplete=()=>resolve();tx.onabort=()=>reject(Error(tx.error?.name==='QuotaExceededError'?'本机图片存储空间不足':tx.error?.message||'图片更改未保存'));tx.onerror=()=>{}});void done.catch(()=>{});return done}
function initial(packageId:string):Collection{return {packageId,revision:0,count:0,bytes:0}}
function checkRevision(current:Collection,revision:unknown){if(!Number.isSafeInteger(revision)||revision!==current.revision)throw Error('图库已在其他窗口更新，请刷新后重试');if(current.revision===Number.MAX_SAFE_INTEGER)throw Error('图片存储修订号已达上限')}
function imageId(value:unknown):asserts value is string{if(typeof value!=='string'||!/^p\.[0-9a-f]{64}$/.test(value))throw Error('图片 ID 无效')}
function jpeg(data:unknown,maxBytes:number,maxSide:number,width:unknown,height:unknown){
  if(typeof data!=='string'||data.length>Math.ceil(maxBytes/3)*4+32||!/^data:image\/jpeg;base64,[A-Za-z0-9+/]+={0,2}$/.test(data))throw Error('只接受有界的 JPEG 展示副本')
  const raw=atob(data.slice(23)),bytes=Uint8Array.from(raw,c=>c.charCodeAt(0))
  if(bytes.length>maxBytes||bytes[0]!==255||bytes[1]!==216||bytes[bytes.length-2]!==255||bytes[bytes.length-1]!==217)throw Error('JPEG 副本格式或容量不正确')
  let p=2,w=0,h=0
  while(p+8<bytes.length){if(bytes[p++]!==255)break;while(bytes[p]===255)p++;const marker=bytes[p++];if(marker===218||marker===217)break;const n=(bytes[p]<<8)|bytes[p+1];if(n<2||p+n>bytes.length)break;if([192,193,194].includes(marker)){h=(bytes[p+3]<<8)|bytes[p+4];w=(bytes[p+5]<<8)|bytes[p+6];break}p+=n}
  if(!w||!h||w>maxSide||h>maxSide||w!==width||h!==height)throw Error('JPEG 尺寸不正确或过大')
  return bytes
}
function dataUrl(bytes:Uint8Array){let binary='';for(let i=0;i<bytes.length;i+=16384)binary+=String.fromCharCode(...bytes.subarray(i,i+16384));return 'data:image/jpeg;base64,'+btoa(binary)}
export async function extensionImages(packageId:string,message:any){
  if(!/^[a-z0-9]+(?:[._-][a-z0-9]+)+$/.test(packageId))throw Error('图片命名空间无效')
  const operation=message.operation
  if(!['list','get','add','remove'].includes(operation))throw Error('图片操作无效')
  if(operation==='get'||operation==='remove')imageId(message.imageId)
  if(operation==='get'&&!['wall','display'].includes(message.variant))throw Error('图片版本无效')
  let prepared:{meta:Metadata;pixels:Pixels}|undefined
  if(operation==='add'){
    const image=message.image
    if(!image||typeof image.name!=='string'||!image.name.trim()||image.name.length>180||/[\x00-\x1f\\/]/.test(image.name))throw Error('图片名称无效')
    const display=jpeg(image.display,imageLimits.displayBytes,imageLimits.displaySide,image.width,image.height),wall=jpeg(image.wall,imageLimits.wallBytes,imageLimits.wallSide,image.wallWidth,image.wallHeight)
    const hash=await crypto.subtle.digest('SHA-256',display),id='p.'+Array.from(new Uint8Array(hash),b=>b.toString(16).padStart(2,'0')).join('')
    prepared={meta:{packageId,id,name:image.name,width:image.width,height:image.height,wallWidth:image.wallWidth,wallHeight:image.wallHeight,bytes:display.byteLength+wall.byteLength,createdAt:Date.now()},pixels:{packageId,id,display,wall}}
  }
  const db=await database(),write=operation==='add'||operation==='remove',tx=db.transaction(['collections','metadata','pixels'],write?'readwrite':'readonly'),done=completion(tx)
  try{
    const collections=tx.objectStore('collections'),metadata=tx.objectStore('metadata'),pixels=tx.objectStore('pixels')
    const current:Collection=(await result(collections.get(packageId)))??initial(packageId)
    if(operation==='list'){
      const items=await result<Metadata[]>(metadata.index('package').getAll(packageId));await done
      items.sort((a,b)=>a.createdAt-b.createdAt||a.id.localeCompare(b.id))
      return {revision:current.revision,bytes:current.bytes,limit:imageLimits.count,images:items.map(({packageId:_,...item})=>item)}
    }
    if(operation==='get'){
      const value=await result<Pixels|undefined>(pixels.get([packageId,message.imageId]));await done
      if(!value)throw Error('图片已被移除');return {id:value.id,dataUrl:dataUrl(message.variant==='wall'?value.wall:value.display)}
    }
    checkRevision(current,message.revision)
    if(operation==='add'){
      const {meta,pixels:value}=prepared!,exists=await result(metadata.get([packageId,meta.id]))
      if(exists){await done;return {id:meta.id,revision:current.revision,duplicate:true}}
      if(current.count>=imageLimits.count)throw Error('首版最多保存 50 张照片')
      if(current.bytes+meta.bytes>imageLimits.packageBytes)throw Error('本图库的展示副本已达 32 MiB，请先移除部分照片')
      const all=await result<Collection[]>(collections.getAll())
      if(all.reduce((sum,c)=>sum+c.bytes,0)+meta.bytes>imageLimits.totalBytes)throw Error('扩展图片总空间已达 128 MiB')
      metadata.add(meta);pixels.add(value);collections.put({...current,revision:current.revision+1,count:current.count+1,bytes:current.bytes+meta.bytes});await done
      return {id:meta.id,revision:current.revision+1,duplicate:false}
    }
    const old=await result<Metadata|undefined>(metadata.get([packageId,message.imageId]))
    if(!old)throw Error('图片已被移除')
    metadata.delete([packageId,message.imageId]);pixels.delete([packageId,message.imageId]);collections.put({...current,revision:current.revision+1,count:current.count-1,bytes:current.bytes-old.bytes});await done
    return {revision:current.revision+1}
  }catch(error){try{tx.abort()}catch{}throw error}
}
