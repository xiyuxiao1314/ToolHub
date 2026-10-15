"""Safety regressions for storage maintenance; fixtures only, no live registry."""
import importlib.util
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

spec=importlib.util.spec_from_file_location('maintenance',Path(__file__).parents[1]/'maintenance.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class MaintenanceTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(prefix='toolhub-maintenance-')
        self.root=Path(self.tmp.name);self.repo=self.root/'repo';self.repo.mkdir();(self.repo/'apps/daemon').mkdir(parents=True)
        (self.repo/'Cargo.toml').write_text('[workspace]\nmembers=["apps/desktop"]\n')
        subprocess.run(['git','init','--quiet',str(self.repo)],check=True)
        self.home=self.root/'user';self.home.mkdir();self.install=self.home/'ToolHub';self.versions=self.install/'versions';self.versions.mkdir(parents=True)
        self.state={'processes':[],'references':[]}
        self.target=self.repo/'target';self.target.mkdir();(self.target/'CACHEDIR.TAG').write_text('cargo cache');(self.target/'cache').write_text('generated')
        (self.repo/'source.rs').write_text('real source')
        for name in ['build-1','build-2','build-3']:self.package(self.versions/name,name)
        (self.install/'current.json').write_text(json.dumps({'path':str(self.versions/'build-3'),'build_id':'build-3'}))
        self.context=m.Context(self.repo,self.install,self.home,state_reader=lambda:self.state)
    def tearDown(self):self.tmp.cleanup()
    def package(self,path,name):
        path.mkdir(parents=True);(path/'package.json').write_text(json.dumps({'schema':'toolhub.package/v1','build_id':name,'created_at':'2026-10-0'+name[-1]+'T00:00:00Z'}));(path/'SHA256SUMS.txt').write_text('fixture')
        for file in ['toolhub.exe','toolhubd.exe','toolhub-desktop.exe']:(path/file).write_bytes(b'fixture only')
    def rows(self,scopes={'build','versions'}):return m.plan(self.context,scopes)
    def test_preview_never_removes_files(self):
        rows=self.rows();self.assertTrue(self.target.exists());self.assertTrue((self.versions/'build-1').exists());self.assertEqual(len([r for r in rows if not r['blocked']]),2)
    def test_removes_only_generated_cache_and_recycles_obsolete_package(self):
        trash=self.root/'trash';trash.mkdir()
        results=m.apply(self.context,self.rows(),{'build','versions'},recycler=lambda p:shutil.move(str(p),trash/p.name))
        self.assertFalse(self.target.exists());self.assertTrue((self.repo/'source.rs').exists());self.assertTrue((self.versions/'build-3').exists());self.assertTrue((self.versions/'build-2').exists());self.assertTrue((trash/'build-1').exists());self.assertEqual({r['result'] for r in results},{'deleted','recycled'})
    def test_current_protected_even_when_oldest(self):
        (self.install/'current.json').write_text(json.dumps({'path':str(self.versions/'build-1'),'build_id':'build-1'}));self.context.refresh();self.assertTrue(all(r['blocked'] for r in self.rows({'versions'})))
    def test_still_configured_old_version_is_kept(self):
        config=self.home/'.cursor/mcp.json';config.parent.mkdir();config.write_text(json.dumps({'command':str(self.versions/'build-1/toolhub.exe')}));self.context.refresh();self.assertTrue(next(r for r in self.rows({'versions'}) if r['path'].endswith('build-1'))['blocked'])
    def test_registered_program_and_relative_artifact_protect_paths(self):
        dbpath=self.home/'.toolhub/registry.sqlite';dbpath.parent.mkdir()
        with sqlite3.connect(dbpath) as db:
            db.execute('CREATE TABLE program_entries(entry_json TEXT)');db.execute('INSERT INTO program_entries VALUES(?)',(json.dumps({'path':str(self.versions/'build-1/toolhub.exe')}),));db.execute('CREATE TABLE artifact_packages(package_json TEXT)');db.execute('INSERT INTO artifact_packages VALUES(?)',(json.dumps({'root':str(self.repo),'files':[{'relative_path':'target/cache'}],'priority':'high'}),))
        db.close()
        self.context.refresh();self.assertTrue(all(r['blocked'] for r in self.rows()))
    def test_command_entry_relative_launch_path_is_retained(self):
        dbpath=self.home/'.toolhub/registry.sqlite';dbpath.parent.mkdir()
        with sqlite3.connect(dbpath) as db:
            db.execute('CREATE TABLE program_entries(entry_json TEXT)');db.execute('INSERT INTO program_entries VALUES(?)',(json.dumps({'kind':'command','cwd':str(self.repo),'command':"& './target/debug/app.exe'",'args':[]}),))
        db.close();self.context.refresh();self.assertTrue(self.rows({'build'})[0]['blocked'])
    def test_new_live_process_invalidates_old_plan(self):
        rows=self.rows({'build'});self.state['processes']=[{'Name':'app.exe','ExecutablePath':str(self.target/'app.exe')}];results=m.apply(self.context,rows,{'build'});self.assertTrue(self.target.exists());self.assertEqual(results[0]['result'],'skipped_changed_context')
    def test_build_in_progress_protects_cache(self):
        self.state['processes']=[{'Name':'cargo.exe'}];self.context.refresh();self.assertTrue(self.rows({'build'})[0]['blocked'])
    def test_tracked_source_inside_target_is_never_removed(self):
        subprocess.run(['git','-C',str(self.repo),'add','--','target/cache'],check=True);self.assertIn('Tracked source',self.rows({'build'})[0]['blocked'])
    def test_no_arbitrary_directory_or_recycle_fallback(self):
        foreign=self.versions/'not-a-package';foreign.mkdir();(foreign/'source.txt').write_text('retain')
        def fail(_):raise OSError('recycle unavailable')
        results=m.apply(self.context,self.rows({'versions'}),{'versions'},recycler=fail);self.assertTrue((self.versions/'build-1').exists());self.assertTrue(foreign.exists());self.assertEqual(results[0]['result'],'failed')
    def test_manifest_change_invalidates_removal(self):
        rows=self.rows({'versions'});path=self.versions/'build-1/package.json';data=json.loads(path.read_text());data['note']='changed';path.write_text(json.dumps(data));results=m.apply(self.context,rows,{'versions'},recycler=lambda _:self.fail('must not recycle changed package'));self.assertEqual(results[0]['result'],'skipped_changed_context')
    def test_report_cannot_overwrite_configuration_or_database(self):
        path=self.home/'configuration.json';path.write_text('{"important":"keep"}')
        with self.assertRaises(m.Unsafe):m.validate_report(path)
        self.assertEqual(path.read_text(),'{"important":"keep"}')
        with self.assertRaises(m.Unsafe):m.validate_report(self.home/'registry.sqlite')
    def test_malformed_package_id_is_ignored(self):
        path=self.versions/'build-1/package.json';data=json.loads(path.read_text());data['build_id']=['bad'];path.write_text(json.dumps(data))
        self.assertFalse(any(r['path'].endswith('build-1') for r in self.rows({'versions'})))
    def test_linked_child_is_refused(self):
        outside=self.root/'outside';outside.mkdir();(outside/'source.txt').write_text('retained')
        link=self.target/'escape'
        if m.os.name=='nt':
            subprocess.run(['powershell','-NoProfile','-NonInteractive','-Command','New-Item -ItemType Junction -Path $env:TOOLHUB_TEST_LINK -Target $env:TOOLHUB_TEST_OUTSIDE | Out-Null'],env={**m.os.environ,'TOOLHUB_TEST_LINK':str(link),'TOOLHUB_TEST_OUTSIDE':str(outside)},check=True,capture_output=True)
        else:link.symlink_to(outside,target_is_directory=True)
        try:
            rows=self.rows({'build'});self.assertTrue(rows[0]['blocked']);m.apply(self.context,rows,{'build'});self.assertTrue((outside/'source.txt').exists())
        finally:
            if m.os.name=='nt':m.os.rmdir(link)
            else:link.unlink()

if __name__=='__main__':unittest.main()
