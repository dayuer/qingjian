# 给回放里每条上屏打标签：retracted（上屏后被撤销）、retyped（上屏后删掉、按 before→after 重打）
import json, sys
L=[json.loads(l) for l in open(sys.argv[1]) if l.strip()]
latest={}; scope_line={}; label={}
for n,x in enumerate(L,1):
    e=x.get('event')
    if e=='commit':
        latest[x['id']]=n; scope_line[x.get('scope') or x.get('keys')]=n
    elif e=='retract':
        if x['of'] in latest: label[latest[x['of']]]='retracted'
    elif e=='retype':
        tgt=latest.get(x['of'])
        prev=scope_line.get(x['before'])
        if prev and tgt and prev<tgt and tgt-prev<40: label.setdefault(prev,'retyped')
json.dump(label,open(sys.argv[2],'w'))
print(len(label), {v:list(label.values()).count(v) for v in set(label.values())})
