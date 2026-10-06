#!/usr/bin/env python3
"""Package one native build without installing or editing user configuration."""
import argparse, hashlib, json, os, pathlib, plistlib, shutil, subprocess, sys, tomllib, zipfile
from licenses import collect

root = pathlib.Path(__file__).resolve().parents[1]
arguments = argparse.ArgumentParser()
arguments.add_argument('--platform', choices=['windows-x64','macos-arm64','macos-x64'], required=True)
arguments.add_argument('--binary-dir', default='target/release')
opts = arguments.parse_args()
version = tomllib.loads((root/'Cargo.toml').read_text())['workspace']['package']['version']
binary_dir = (root/opts.binary_dir).resolve()
output = root/'dist'/'release'/opts.platform
output.mkdir(parents=True, exist_ok=False)
suffix = '.exe' if opts.platform.startswith('windows') else ''
names = ['toolhub-desktop', 'toolhubd', 'toolhub']
for name in names:
    if not (binary_dir/(name+suffix)).is_file(): raise SystemExit('Missing native binary: '+name)
reported = subprocess.check_output([str(binary_dir/('toolhub'+suffix)), '--version'], text=True).strip()
if reported != 'toolhub '+version: raise SystemExit('CLI and source version mismatch')
license_cache=output/'license-data'
license_cache.mkdir()
collect(root,license_cache)

def copy_assets(destination, desktop=True):
    for name in ['schemas','resources','skills']:
        source=root/name
        if source.is_dir(): shutil.copytree(source,destination/name)
    for name in ['LICENSE','NOTICE','README.md']:
        shutil.copy2(root/name,destination/name)
    shutil.copytree(license_cache/'THIRD-PARTY-LICENSES',destination/'THIRD-PARTY-LICENSES')
    shutil.copy2(license_cache/'DEPENDENCIES.json',destination/'DEPENDENCIES.json')
    shutil.copy2(root/'docs/releases'/('v'+version+'.md'),destination/'RELEASE-NOTES.md')
    (destination/'package.json').write_text(json.dumps({'schema':'toolhub.release/v1','version':version,'platform':opts.platform,'signing':'unsigned' if suffix else 'ad-hoc; not notarized','binaries':[(n+suffix) for n in names if desktop or n!='toolhub-desktop']},indent=2)+'\n')

def zip_folder(folder, name):
    target=output/name
    with zipfile.ZipFile(target,'x',compression=zipfile.ZIP_DEFLATED,compresslevel=9,strict_timestamps=False) as archive:
        for item in sorted(folder.rglob('*')):
            archive.write(item,item.relative_to(folder.parent))
    return target

assets=[]
if suffix:
    folder=output/('ToolHub-'+version+'-windows-x64')
    folder.mkdir()
    for name in names: shutil.copy2(binary_dir/(name+suffix),folder/(name+suffix))
    copy_assets(folder)
    shutil.copy2(root/'scripts/install-local.ps1',folder/'install-local.ps1')
    # The local installer expects this metadata and a complete file checksum manifest.
    (folder/'package.json').write_text(json.dumps({'schema':'toolhub.package/v1','build_id':'release-'+version.replace('.','-'),'version':reported,'configuration':'release','unsigned':True,'binaries':[n+suffix for n in names]},indent=2)+'\n')
    sums=['{}  {}'.format(hashlib.sha256(p.read_bytes()).hexdigest(),p.relative_to(folder).as_posix()) for p in sorted(folder.rglob('*')) if p.is_file()]
    (folder/'SHA256SUMS.txt').write_text('\n'.join(sums)+'\n')
    assets.append(zip_folder(folder,'ToolHub-v'+version+'-windows-x64.zip'))
else:
    if sys.platform!='darwin': raise SystemExit('macOS assets must be generated on macOS')
    folder=output/('ToolHub-'+version+'-'+opts.platform)
    app=folder/'ToolHub.app'
    executable=app/'Contents/MacOS'
    resources=app/'Contents/Resources'
    executable.mkdir(parents=True)
    resources.mkdir(parents=True)
    for name in names:
        shutil.copy2(binary_dir/name,executable/name)
        (executable/name).chmod(0o755)
    # Keep siblings next to the paired executables; all clients share this layout.
    copy_assets(executable)
    iconset=output/'ToolHub.iconset'
    iconset.mkdir()
    for size in [16,32,128,256,512]:
        for scale in [1,2]:
            target=iconset/f'icon_{size}x{size}{"@2x" if scale==2 else ""}.png'
            subprocess.run(['sips','-z',str(size*scale),str(size*scale),str(root/'apps/desktop/icons/icon.png'),'--out',str(target)],check=True,stdout=subprocess.DEVNULL)
    subprocess.run(['iconutil','-c','icns',str(iconset),'-o',str(resources/'ToolHub.icns')],check=True)
    info={'CFBundleName':'ToolHub','CFBundleDisplayName':'ToolHub','CFBundleIdentifier':'dev.toolhub.desktop','CFBundleExecutable':'toolhub-desktop','CFBundlePackageType':'APPL','CFBundleShortVersionString':version.split('-')[0],'CFBundleVersion':version.split('-')[0],'CFBundleIconFile':'ToolHub.icns','LSMinimumSystemVersion':'13.0','NSHighResolutionCapable':True,'NSAppleEventsUsageDescription':'ToolHub opens Terminal at a directory you select.'}
    (app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
    for name in names: subprocess.run(['codesign','--force','--sign','-',str(executable/name)],check=True)
    subprocess.run(['codesign','--force','--deep','--sign','-',str(app)],check=True)
    subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
    # Include explicit install/help documents outside the application too.
    for name in ['LICENSE','README.md']: shutil.copy2(root/name,folder/name)
    assets.append(zip_folder(folder,'ToolHub-v'+version+'-'+opts.platform+'.zip'))
    image_source=output/'dmg-content'
    image_source.mkdir()
    shutil.copytree(app,image_source/'ToolHub.app')
    (image_source/'Applications').symlink_to('/Applications')
    dmg=output/('ToolHub-v'+version+'-'+opts.platform+'.dmg')
    subprocess.run(['hdiutil','create','-volname','ToolHub','-srcfolder',str(image_source),'-ov','-format','UDZO',str(dmg)],check=True)
    assets.append(dmg)

cli=output/('ToolHub-CLI-'+version+'-'+opts.platform)
cli.mkdir()
for name in ['toolhub','toolhubd']: shutil.copy2(binary_dir/(name+suffix),cli/(name+suffix))
copy_assets(cli,desktop=False)
assets.append(zip_folder(cli,'ToolHub-CLI-v'+version+'-'+opts.platform+'.zip'))
print(json.dumps({'version':version,'platform':opts.platform,'assets':[str(p) for p in assets]},indent=2))
