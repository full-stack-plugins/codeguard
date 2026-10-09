from pathlib import Path
import os,subprocess,hashlib,json,shutil,time,signal
BASE=Path('/workspace/guard-implementation-ledger/codeguard-native-parity-capture')
PLUGIN=Path('/workspace/guard-implementation/codeguard-plugin')
OLD=Path('/workspace/guard-implementation-ledger/codeguard-ruff-native-capture/codeguard-f5661d5')
RUFF=Path('/workspace/guard-toolchain/python-native/bin/ruff')
NODE=Path('/opt/codex/runtimes/codex-primary-runtime/dependencies/node/bin')
ENV={'PATH':f'{RUFF.parent}:/workspace/guard-toolchain/hadolint-2.12.0:{NODE}:/usr/bin:/bin','LANG':'C.UTF-8','PYTHONDONTWRITEBYTECODE':'1','HOME':str(BASE/'home'),'npm_config_cache':str(BASE/'npm-cache'),'npm_config_userconfig':'/dev/null','npm_config_globalconfig':str(BASE/'empty-global-npmrc'),'npm_config_registry':'https://registry.npmjs.org'}
ENV.update({k:os.environ[k] for k in ['HTTP_PROXY','HTTPS_PROXY','http_proxy','https_proxy','NODE_EXTRA_CA_CERTS','SSL_CERT_FILE'] if k in os.environ})
ENV.update({'npm_config_fetch_retries':'0','npm_config_fetch_timeout':'20000'})
BASE.mkdir(exist_ok=True);(BASE/'home').mkdir(exist_ok=True)
def sha(b):return hashlib.sha256(b).hexdigest()
def snapshot(root):return {str(p.relative_to(root)):sha(p.read_bytes()) for p in sorted(root.rglob('*')) if p.is_file()}
def project(name,files):
 p=BASE/'projects'/name;p.mkdir(parents=True,exist_ok=True)
 if (p/'.ruff_cache').exists():shutil.rmtree(p/'.ruff_cache')
 for n,b in files.items():f=p/n;f.parent.mkdir(parents=True,exist_ok=True);f.write_text(b)
 return p
def record(name,argv,root,stdin_dir=False,env=ENV):
 out=BASE/'records'/name;out.mkdir(parents=True,exist_ok=True);before=snapshot(root)
 fd=os.open(root,os.O_RDONLY|os.O_DIRECTORY) if stdin_dir else subprocess.DEVNULL
 start=time.time()
 try:r=subprocess.run([str(a) for a in argv],stdin=fd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,cwd=root,timeout=90)
 finally:
  if stdin_dir:os.close(fd)
 (out/'stdout').write_bytes(r.stdout);(out/'stderr').write_bytes(r.stderr)
 manifest={'name':name,'argv':[str(a) for a in argv],'cwd':str(root),'exit':r.returncode,'stdoutSha256':sha(r.stdout),'stderrSha256':sha(r.stderr),'before':before,'after':snapshot(root),'stdin':'directory FD; real read EISDIR' if stdin_dir else 'null','elapsedSeconds':round(time.time()-start,3)}
 (out/'capture.json').write_text(json.dumps(manifest,indent=2)+'\n');print(name,r.returncode,len(r.stdout),len(r.stderr),flush=True);return r
if __name__=='__main__':
 import sys
 if sys.argv[1]=='legacy':
  p=project('legacy-check',{'app.py':'import os\n','ruff.toml':'[lint]\nselect=["F401"]\n','requirements.txt':'','demo.ex':'defmodule Demo do\nend\n'})
  record('legacy-check-mixed',[PLUGIN/'bin/codeguard','check','--lang','python,elixir','--quiet',p],p)
  p=project('legacy-docker',{'Dockerfile':'FROM ubuntu:latest\nRUN apt-get update\n'})
  record('legacy-docker-mixed',[PLUGIN/'bin/codeguard','dockerfile','--json',p],p)
  p=project('legacy-cve',{'package.json':'{"name":"guard-native-parity-generated","version":"1.0.0","private":true,"dependencies":{"lodash":"4.17.20"}}\n','requirements.txt':'requests==2.19.0\n'})
  r=record('npm-lock-generation',[NODE/'npm','install','--package-lock-only','--ignore-scripts','--no-audit','--no-fund'],p)
  if r.returncode==0:record('legacy-cve-mixed',[PLUGIN/'bin/codeguard','cve','--json',p],p)
 elif sys.argv[1]=='native':
  binary=Path(sys.argv[2]);label=sys.argv[3]
  for case,src in [('format-clean','x = 1\n'),('format-bad','x=1\n')]:
   p=project(case,{'app.py':src});record(label+'-'+case,[binary,'format','check','python',p,'--tool',f'ruff={RUFF}','--format=json'],p)
  p=project('native-query',{'app.py':'x = 1\n'})
  record(label+'-query',[binary,'--version','--format=json'],p)
  record(label+'-usage',[binary,'lint','python',p,'--unsupported-parity-flag'],p)
  record(label+'-io-error',[binary,'hook','execute',p,'--format=json','--timeout','1s'],p,stdin_dir=True)
  for case,src,config,tool in [('lint-clean','x = 1\n','[lint]\nselect=["F401"]\n',RUFF),('lint-bad','import os\n','[lint]\nselect=["F401"]\n',RUFF),('lint-missing','import os\n','[lint]\nselect=["F401"]\n',BASE/'absent-ruff'),('lint-error','import os\n','[lint\n',RUFF)]:
   p=project(case,{'app.py':src,'ruff.toml':config});record(label+'-'+case,[binary,'lint','python',p,'--file','app.py','--ruff-tool',tool,'--format=json','--timeout','10s'],p)
