import {ToolhubClient, nodeCliTransport, ResolveResult, ExecutionResult} from '../src/index.js';
const client=new ToolhubClient(nodeCliTransport());
const resolve:Promise<ResolveResult>=client.resolveCapability('language.python.execute',{require_trust:'known'});
const execute:Promise<ExecutionResult>=client.execute({instance_id:'fixture',args:[]});
client.call('registry.search',{query:'python'});
// @ts-expect-error query must be string
client.call('registry.search',{query:42});
// @ts-expect-error unknown method has no typed contract
client.call('unknown.method',{});
// @ts-expect-error args are string array
client.execute({instance_id:'fixture',args:[42]});
void resolve;void execute;
client.call('extension.list',{}).then(rows=>{const enabled:boolean=rows[0].enabled;void enabled;});
client.call('extension.set_enabled',{id:'community.blender',enabled:false,expected_digest:'fixture'});
// @ts-expect-error extension changes must pin the current digest
client.call('extension.set_enabled',{id:'community.blender',enabled:false});
// @ts-expect-error enabled is boolean
client.call('extension.set_enabled',{id:'community.blender',enabled:'false',expected_digest:'fixture'});

const background = client.execute({instance_id:"fixture",args:[],background:true});
background.then(result => { if(result.status === "running") { const id:string=result.execution_id;void id; } });
client.call("program.select",{proposal_ids:["fixture"]});
client.call("execute.approval_status",{request_id:"fixture"}).then(r=>{const status:string=r.status;void status});

client.call('skill.preview',{manifest:{},instructions:'Workflow'});
client.call('skill.register',{path:'fixture.json',if_absent:true});
// @ts-expect-error preview instructions must be text
client.call('skill.preview',{manifest:{},instructions:42});
