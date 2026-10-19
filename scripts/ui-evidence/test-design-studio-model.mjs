import fs from 'node:fs'
import vm from 'node:vm'
import assert from 'node:assert/strict'
const context=vm.createContext({});vm.runInContext(fs.readFileSync(new URL('../../extensions/design-studio/preview.js',import.meta.url),'utf8'),context)
const api=context.DesignStudio,base=JSON.parse(JSON.stringify(api.defaultConfig)),copy=()=>JSON.parse(JSON.stringify(base))
assert.equal(api.presets.length,8)
for(const preset of api.presets)assert.equal(api.validate(preset.config).schema,'toolhub.design/v1')
for(const mutate of [c=>c.script='alert(1)',c=>delete c.title,c=>c.layout='unknown',c=>c.effects=['expand','expand'],c=>c.modules=[],c=>c.radius=Infinity,c=>c.gap=-5,c=>c.accent='url(file:///secret)',c=>c.title='x'.repeat(49),c=>c.dark='yes',c=>c.performance='virtual',c=>c.dataSize=10000,c=>c.modules=['content','__proto__']]){const value=copy();mutate(value);assert.throws(()=>api.validate(value))}
const dynamic=copy();dynamic.layout='list';dynamic.performance='virtual';dynamic.dataSize=10000;dynamic.effects.push('drag');assert.throws(()=>api.validate(dynamic));assert(!api.normalize(dynamic).config.effects.includes('drag'))
for(const layout of Object.keys(api.lists.layout))for(const navigation of Object.keys(api.lists.navigation))for(const appearance of Object.keys(api.lists.appearance)){const value={...copy(),layout,navigation,appearance};const normalized=api.normalize(value);api.validate(normalized.config);assert(api.exportHtml(normalized.config,'').includes('<!doctype html>'))}
const escaped=copy();escaped.title='</title><script>alert(1)</script>';const output=api.exportHtml(api.validate(escaped),'');assert(!output.includes('</title><script>alert(1)</script>'));assert(output.includes('&lt;/title&gt;'))
console.log('PASS preset/model validation, bounds, unsafe inputs, constraints, all structure/navigation/style combinations and HTML escaping')
