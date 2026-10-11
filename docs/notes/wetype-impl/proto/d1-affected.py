import re,itertools
src=open('/Users/liyuqing/sproot/qingjian/crates/qingjian-core/src/parser/syllable.rs').read()
body=src.split('pub const SYLLABLES')[1].split('];')[0]
S=set(re.findall(r'"([a-z]+)"',body))
P=set(s[:i] for s in S for i in range(1,len(s)))  # proper prefixes
INI=[("zh","z"),("ch","c"),("sh","s"),("n","l"),("f","h"),("l","r")]
FIN=[("ang","an"),("eng","en"),("ing","in")]
NAMES={"zh":"z_zh","ch":"c_ch","sh":"s_sh","n":"n_l","f":"f_h","l":"l_r","ang":"an_ang","eng":"en_eng","ing":"in_ing"}
def apply_once(form, rules):
    out=[]
    for a,b in INI:
        if NAMES[a] not in rules: continue
        if form.startswith(a): out.append(b+form[len(a):])
        elif form.startswith(b): out.append(a+form[len(b):])
    for a,b in FIN:
        if NAMES[a] not in rules: continue
        if form.endswith(a): out.append(form[:-len(a)]+b)
        elif form.endswith(b): out.append(form[:-len(b)]+a)
    return [f for f in out if f and f!=form]
ALL=list(NAMES.values())
def sources(rules, depth):
    # strings reachable from a legal syllable by <=depth inverse swaps (swaps are involutions-ish), not legal themselves
    res={}
    frontier={t:(t,) for t in S}
    seen=dict(frontier)
    for d in range(depth):
        nxt={}
        for f,chain in frontier.items():
            for g in apply_once(f,rules):
                if g not in seen:
                    seen[g]=chain+(g,); nxt[g]=chain+(g,)
        frontier=nxt
    for g,chain in seen.items():
        if g not in S and len(g)<=7:
            res[g]=chain
    return res
# the swap rules are not exact involutions (n_l then l_r chains), so compute forward check: source s ok iff some apply chain from s hits legal
def forward_targets(s,rules,depth):
    tg=set(); fr={s}
    for d in range(depth):
        nf=set()
        for f in fr:
            for g in apply_once(f,rules):
                if g in S: tg.add(g)
                nf.add(g)
        fr=nf
    return tg
print("=== 单条规则：源拼写不合法、一步替换得到合法音节（qingjian 现行 apply_once 语义）")
for r in ALL:
    cands=sources([r],1)
    ok={}
    for s in cands:
        t=forward_targets(s,[r],1)
        if t: ok[s]=sorted(t)
    items=sorted(ok.items())
    print(f"{r} ({len(items)}):", " ".join(f"{s}→{'/'.join(t)}" for s,t in items))
print()
print("=== 全开（ALL）时额外需要两步（经过非法中间态）才合法的源：")
one=set(); 
c1=sources(ALL,1)
for s in c1:
    if forward_targets(s,ALL,1): one.add(s)
c2=sources(ALL,2)
two={}
for s in c2:
    if s in one: continue
    t=forward_targets(s,ALL,2)
    if t: two[s]=sorted(t)
print(len(two), " ".join(f"{s}→{'/'.join(t)}" for s,t in sorted(two.items())))
print()
print("=== 全开时单步源总数", len(one))
# which of those sources are proper prefixes of legal syllables (parser would already give partial at end) / or are legal-prefix-free
print("=== 单步源里『不是任何合法音节前缀』的（输入末尾时解析器连残缺音节都不给）：")
for r in ALL:
    xs=[s for s in sources([r],1) if forward_targets(s,[r],1) and s not in P]
    print(r, len(xs), " ".join(sorted(xs)))
