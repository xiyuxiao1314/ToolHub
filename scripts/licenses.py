"""Collect dependency declarations and available license texts for a native release."""
import json, pathlib, shutil, subprocess

def collect(root, destination):
    target = subprocess.check_output(['rustc','-vV'], text=True, encoding='utf-8').split('host: ')[1].splitlines()[0]
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--format-version','1','--locked','--filter-platform',target],cwd=root,text=True, encoding='utf-8'))
    license_root=destination/'THIRD-PARTY-LICENSES'
    license_root.mkdir()
    inventory=[]
    for package in sorted(metadata['packages'],key=lambda p:(p['name'],p['version'])):
        if package['source'] is None: continue
        source=pathlib.Path(package['manifest_path']).parent
        folder=license_root/(package['name']+'-'+package['version'])
        candidates=[p for p in source.iterdir() if p.is_file() and p.name.lower().startswith(('license','licence','copying','copyright','notice'))]
        if package.get('license_file'):
            file=(source/package['license_file']).resolve()
            if file.is_file() and source in file.parents and file not in candidates: candidates.append(file)
        if candidates:
            folder.mkdir()
            for file in candidates:
                if file.stat().st_size<=1_048_576: shutil.copy2(file,folder/file.name)
        inventory.append({'ecosystem':'cargo','name':package['name'],'version':package['version'],'declared_license':package.get('license'),'source':package['source'],'license_texts':[p.name for p in candidates]})
    # The browser bundle includes production React dependencies; build tools are not shipped.
    installed=root/'apps/desktop/ui/node_modules'
    queue=['react','react-dom'];seen=set()
    while queue:
        name=queue.pop()
        if name in seen: continue
        seen.add(name)
        source=installed/name
        package=json.loads((source/'package.json').read_text(encoding='utf-8'))
        queue.extend(package.get('dependencies',{}))
        folder=license_root/('npm-'+name.replace('/','-')+'-'+package['version'])
        folder.mkdir()
        texts=[]
        for file in source.iterdir():
            if file.is_file() and file.name.lower().startswith(('license','licence','copying','notice')):
                shutil.copy2(file,folder/file.name);texts.append(file.name)
        inventory.append({'ecosystem':'npm','name':name,'version':package['version'],'declared_license':package.get('license'),'license_texts':texts})
    (destination/'DEPENDENCIES.json').write_text(json.dumps(inventory,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
