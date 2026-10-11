"""数 input-log.jsonl 里按现规则 / 新规则会触发多少次云联想请求（不发请求）。
用法: python3 -I sim.py <input-log.jsonl> <syllables.txt>"""
import json, sys, random
log, sylf = sys.argv[1], sys.argv[2]
SYL = set(l.strip() for l in open(sylf) if l.strip())
MAXS = max(map(len, SYL))

def full_pinyin(s):
    # 能否整串切成完整音节
    ok = [False]*(len(s)+1); ok[0] = True
    for i in range(len(s)):
        if not ok[i]: continue
        for j in range(i+1, min(len(s), i+MAXS)+1):
            if s[i:j] in SYL: ok[j] = True
    return ok[len(s)]

comps = []   # (letters, ms, retypes)
prev = None
retype_pending = 0
for line in open(log, encoding='utf8'):
    e = json.loads(line)
    ev = e.get('event')
    if ev == 'retype':
        retype_pending += 1; continue
    if ev != 'commit': continue
    if e.get('english') or e.get('source') not in ('word', 'sentence', 'cloud_sentence'):
        prev = None; continue
    scope = e['scope'].replace("'", '').lower()
    if not scope.isalpha():
        prev = None; continue
    cont = prev is not None and prev[0] != prev[1] and prev[0].endswith(scope) and len(scope) < len(prev[0])
    prev = (scope, e['keys'].replace("'", '').lower())
    if cont:
        continue
    comps.append((scope, e.get('ms', 0), retype_pending)); retype_pending = 0

GAP = 100        # 打字键间隔（毫秒）：取日志 ms/键 的 p25，偏保守
DEBOUNCE = 300
def run(p_pause, seed=1):
    rnd = random.Random(seed)
    cur_req = cur_net = 0
    new_req = new_net = 0
    cur_cache, new_cache = [], []
    for letters, ms, retypes in comps:
        n = len(letters)
        final_pause = ms - (n-1)*GAP
        # 组句中途的停顿点：在第 k 个键之后停顿 ≥300ms（随机，概率 p_pause）
        mids = [k for k in range(2, n) if rnd.random() < p_pause]
        points = mids + ([n] if final_pause >= DEBOUNCE else [])
        # 现规则：每个停顿点只要 ≥2 字母就发；退格后的停顿也发；缓存键≈字母串（实际还含候选，偏乐观）
        cur_points = [k for k in points if k >= 2] + [n]*retypes
        for k in cur_points:
            cur_req += 1
            key = letters[:k]
            if key in cur_cache: continue
            cur_net += 1; cur_cache.append(key); cur_cache[:] = cur_cache[-64:]
        # 新规则
        last_sent = 0
        for k in points:
            s = letters[:k]
            if k < 4: continue                                  # N1 至少 4 个字母
            if k <= 6 and full_pinyin(s): continue              # N2 短而完整的全拼：本地够了
            if last_sent and k - last_sent < 3: continue        # N5 同一段组句里至少多 3 个字母才再问
            new_req += 1; last_sent = k
            if s in new_cache: continue                         # N4 缓存键只含上下文+字母（日志无上下文，按字母算）
            new_net += 1; new_cache.append(s); new_cache[:] = new_cache[-256:]
        # N3 退格不发
    return cur_req, cur_net, new_req, new_net

chars = 0
print('组句段数', len(comps))
for p in (0.0, 0.1, 0.2):
    c, cn, nw, nn = run(p)
    print(f'p_pause={p:.1f}  现规则 触发 {c} 次/实发 {cn} 次   新规则 触发 {nw} 次/实发 {nn} 次   降幅 {1-nn/cn:.0%}')
