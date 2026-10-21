import assert from 'node:assert/strict'
import { extensionState } from '../../apps/desktop/ui/src/workbench/extensionStorage.ts'
class MemoryStorage {
  values=new Map();getItem(key){return this.values.get(key)??null}setItem(key,value){this.values.set(key,value)}
}
const store=new MemoryStorage()
assert.deepEqual(extensionState(store,'community.one','get'),{revision:0,data:null})
assert.deepEqual(extensionState(store,'community.one','set',0,{presets:['我的预设']}),{revision:1,data:{presets:['我的预设']}})
assert.deepEqual(extensionState(store,'community.two','get'),{revision:0,data:null})
assert.throws(()=>extensionState(store,'community.one','set',0,{presets:[]}))
assert.throws(()=>extensionState(store,'community.one','set',1,'中'.repeat(350000)))
assert.deepEqual(extensionState(store,'community.one','get').data,{presets:['我的预设']})
assert.throws(()=>extensionState(store,'community.one','delete',1,null))
assert.throws(()=>extensionState(store,'community.one','set',1,undefined))
assert.equal(extensionState(store,'large','set',0,'中'.repeat(12000)).revision,1)
store.setItem('th.extension-state.v1.bad','{broken')
assert.throws(()=>extensionState(store,'bad','get'))
assert.equal(store.getItem('th.extension-state.v1.bad'),'{broken')
console.log('PASS presentation storage isolation, revisions, UTF-8 quota, invalid operations and corrupt-data preservation')
