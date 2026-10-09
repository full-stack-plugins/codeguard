import importlib.util,pathlib,subprocess,time,os,signal,json,re
spec=importlib.util.spec_from_file_location('capture','/workspace/guard-implementation-ledger/capture-codeguard-native-parity.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
root=m.project('native-cancel',{'app.py':'import os\n'*20000,'ruff.toml':'[lint]\nselect=["F401"]\n'})
for label,binary in [('old',m.OLD),('new',m.BASE/'bin/codeguard-ff4efa9')]:
 out=m.BASE/'records'/(label+'-cancel');out.mkdir(parents=True,exist_ok=True)
 argv=[str(binary),'lint','python',str(root),'--file','app.py','--ruff-tool',str(m.RUFF),'--format=json','--timeout','30s'];before=m.snapshot(root)
 trace=out/'process.trace';trace.write_text('')
 p=subprocess.Popen(['strace','-f','-s','4096','-e','trace=process,signal','-o',str(trace),*argv],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=m.ENV,cwd=root)
 sent=False;start=time.monotonic();observed=[];native=None
 while p.poll() is None and time.monotonic()-start<12:
  text=trace.read_text() if trace.exists() else ''
  for line in text.splitlines():
   if 'execve(' in line and str(binary) in line: native=int(line.split()[0])
   if 'execve(' in line and str(m.RUFF) in line and '"check"' in line and '--show-settings' not in line and '--show-files' not in line:
    observed.append(line);os.kill(native,signal.SIGINT);sent=True;break
  if sent:break
  time.sleep(.001)
 if not sent:
  p.kill();a,b=p.communicate();raise AssertionError((label,'no actual Ruff check observed',p.returncode,a[:100],b[:100]))
 stdout,stderr=p.communicate(timeout=12);(out/'stdout').write_bytes(stdout);(out/'stderr').write_bytes(stderr)
 meta={'name':label+'-cancel','argv':argv,'captureWrapper':'strace -f process,signal; raw trace retained','cwd':str(root),'exit':p.returncode,'stdoutSha256':m.sha(stdout),'stderrSha256':m.sha(stderr),'before':before,'after':m.snapshot(root),'signal':'SIGINT after real Ruff check exec observed','observedExec':observed}
 (out/'capture.json').write_text(json.dumps(meta,indent=2)+'\n');print(label,p.returncode,len(stdout),flush=True)
 assert p.returncode==130 and json.loads(stdout)['exit_code']==130
