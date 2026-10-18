#!/usr/bin/env python3
"""Check publication artifacts, version pairing and extension resources."""
import argparse,json,pathlib,sys,tomllib,zipfile
root=pathlib.Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser();parser.add_argument('--platform',required=True,choices=['windows-x64','macos-arm64','macos-x64']);opts=parser.parse_args()
version=tomllib.loads((root/'Cargo.toml').read_text(encoding='utf-8'))['workspace']['package']['version']
folder=root/'dist/release'/opts.platform
archives=sorted(folder.glob('*.zip'));assert len(archives)==2,archives
reports=[]
for path in archives:
 with zipfile.ZipFile(path) as archive:
  names=archive.namelist();assert archive.testzip() is None,'corrupt archive'
  assert all(not n.startswith(('/', '\\')) and '..' not in pathlib.PurePosixPath(n).parts for n in names),'unsafe archive paths'
  assert not any(n.endswith(('.sqlite','.sqlite-wal','.sqlite-shm','.env','config.toml','mimocode.jsonc')) for n in names),'private/runtime data in package'
  top=names[0].split('/')[0]
  cli=path.name.startswith('ToolHub-CLI-')
  assetroot=top+'/' if opts.platform=='windows-x64' or cli else top+'/ToolHub.app/Contents/Resources/'
  code=top+'/' if opts.platform=='windows-x64' or cli else top+'/ToolHub.app/Contents/MacOS/'
  suffix='.exe' if opts.platform=='windows-x64' else ''
  for binary in (['toolhub','toolhubd'] if cli else ['toolhub','toolhubd','toolhub-desktop']):assert code+binary+suffix in names,binary
  for module in ['tools','programs','artifacts','envs','skills','market','tasks']:assert assetroot+'extensions/official/'+module+'.toolhub-extension.json' in names,module
  for sample in ['ffmpeg','graphite','workspace','blender']:assert assetroot+'extensions/examples/'+sample+'.toolhub-extension.json' in names,sample
  for resource in ['extensions/sdk/runtime.js','skills/toolhub-integration/SKILL.md','skills/toolhub-integration/references/setup.md','schemas/extension.schema.json','docs/development/PROJECT_DEPENDENCIES.md','DEPENDENCIES.json','RELEASE-NOTES.md']:assert assetroot+resource in names,resource
  metadata=json.loads(archive.read(assetroot+'package.json'));assert metadata['version'] in [version,'toolhub '+version],metadata
  assert 'ffmpeg.exe' not in [n.split('/')[-1].lower() for n in names]
  assert 'electron.exe' not in [n.split('/')[-1].lower() for n in names]
  reports.append({'archive':path.name,'files':len(names),'bytes':path.stat().st_size,'extensions_present':True,'paired_binaries':True,'no_private_runtime_data':True})
if opts.platform.startswith('macos'):assert len(list(folder.glob('*.dmg')))==1
(folder/'verification.json').write_text(json.dumps({'passed':True,'platform':opts.platform,'version':version,'archives':reports},indent=2)+'\n',encoding='utf-8')
print(json.dumps({'passed':True,'platform':opts.platform,'archives':len(archives)}))
