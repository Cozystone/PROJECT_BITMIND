"""Secondary aggregation over the formal seeds (read-only): life status and cost, clipping and
homeostasis per arm, re-adaptation across structural changes, order probe, control worlds.
Usage: python scripts/aggregate_formal.py results/formal 101-110 [path/to/life-lab]
Writes <root>/secondary.json and prints a summary. Primary statistics come from `life-lab gate`."""
import json, os, statistics, subprocess, sys

root = sys.argv[1]
a, b = sys.argv[2].split('-')
seeds = list(range(int(a), int(b) + 1))
lab = sys.argv[3] if len(sys.argv) > 3 else os.path.join('target', 'release', 'life-lab.exe')
ARMS = ['full', 'frozen', 'readout', 'homeo', 'freeze_at', 'fb_wt', 'ltc', 'full_shuffled', 'full_iid', 'full_perm']

def med(v):
    v = [x for x in v if x is not None and x == x]
    return statistics.median(v) if v else float('nan')

out = {'seeds': seeds, 'lives': {}, 'cost': {}, 'clipping': {}, 'readapt': {}, 'order': {}, 'variants': {}}
status = {}
elapsed, tps, tbytes, ckbytes = [], [], 0, 0
for s in seeds:
    for arm in ARMS:
        p = os.path.join(root, f'seed{s}', arm, 'summary.json')
        if not os.path.exists(p):
            status.setdefault(arm, []).append('missing')
            continue
        j = json.load(open(p))
        status.setdefault(arm, []).append(j['status'])
        elapsed.append(j['elapsed_s']); tps.append(j['ticks_per_s']); tbytes += j['trace_bytes']
        d = os.path.join(root, f'seed{s}', arm)
        ckbytes += sum(os.path.getsize(os.path.join(d, f)) for f in os.listdir(d) if f.startswith('ckpt_'))
out['lives'] = {arm: {'ok': st.count('ok'), 'numerically_invalid': st.count('numerically_invalid'), 'missing': st.count('missing')} for arm, st in status.items()}
out['cost'] = {'lives': len(elapsed), 'median_elapsed_s': med(elapsed), 'median_ticks_per_s': med(tps), 'trace_gb': tbytes / 1e9, 'checkpoint_gb': ckbytes / 1e9}

# clipping, homeostasis, final-window surprise per arm (last 100k window), via life-lab summary
for arm in ['full', 'readout', 'frozen', 'homeo', 'fb_wt', 'ltc']:
    rows = []
    for s in seeds:
        d = os.path.join(root, f'seed{s}', arm)
        if not os.path.exists(os.path.join(d, 'summary.json')):
            continue
        if not os.path.exists(os.path.join(d, 'summary_windows.json')):
            subprocess.run([lab, 'summary', '--run', d, '--window', '100000'], stdout=subprocess.DEVNULL, check=True)
        w = json.load(open(os.path.join(d, 'summary_windows.json')))
        last = w[-1]
        rows.append({'clips_hidden_last': last['clips_hidden'], 'clips_out_last': last['clips_out'],
                     'clips_hidden_total': sum(x['clips_hidden'] for x in w), 'clips_out_total': sum(x['clips_out'] for x in w),
                     'homeo_saturated_last': last.get('homeo_saturated_unit_applications', 0), 'homeo_clips_total': sum(x.get('homeo_clips', 0) for x in w),
                     'dw_hidden_last': last['dw_hidden_norm'], 'dw_out_last': last['dw_out_norm'], 'homeo_dw_last': last['homeo_dw_norm'],
                     'r_rms_last': last['state_mean_rrms'], 'x_max_life': max(x['state_max_x'] for x in w),
                     's_here_last': last['mean_surprise'][0], 's_forced_last': last['s_here_forced'], 's_free_last': last['s_here_free'],
                     'forced_frac_last': last['n_here_forced'] / max(1, last['n_here_forced'] + last['n_here_free']),
                     'clip_trend': (sum(x['clips_hidden'] for x in w[5:]) - sum(x['clips_hidden'] for x in w[:5]))})
    if rows:
        out['clipping'][arm] = {k: med([r[k] for r in rows]) for k in rows[0]}
        out['clipping'][arm]['n'] = len(rows)
        out['clipping'][arm]['clips_growing_in_second_half'] = sum(1 for r in rows if r['clip_trend'] > 0)

# re-adaptation
cases = []
for s in seeds:
    d = os.path.join(root, f'seed{s}')
    for f in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        if f.startswith('readapt_') and f.endswith('.json'):
            rs = {r['label']: r for r in json.load(open(os.path.join(d, f)))}
            if all(k in rs for k in ('continue', 'fresh', 'frozen')):
                cases.append({'seed': s, 't_c': rs['continue']['t_c'], 'n': rs['continue']['n_events'],
                              'continue': rs['continue']['mean_30k_50k'], 'fresh': rs['fresh']['mean_30k_50k'], 'frozen': rs['frozen']['mean_30k_50k'],
                              'converged': {k: rs[k]['converged'] for k in ('continue', 'fresh', 'frozen')},
                              'slope': {k: rs[k]['slope_last_20k_per_10k'] for k in ('continue', 'fresh', 'frozen')}})
valid = [c for c in cases if all(c[k] == c[k] for k in ('continue', 'fresh', 'frozen'))]
out['readapt'] = {
    'cases': len(cases), 'valid_cases': len(valid),
    'median_mean_30k_50k': {k: med([c[k] for c in valid]) for k in ('continue', 'fresh', 'frozen')},
    'continue_below_fresh': sum(1 for c in valid if c['continue'] < c['fresh']),
    'continue_below_frozen': sum(1 for c in valid if c['continue'] < c['frozen']),
    'fresh_below_frozen': sum(1 for c in valid if c['fresh'] < c['frozen']),
    'converged_counts': {k: sum(1 for c in valid if c['converged'][k]) for k in ('continue', 'fresh', 'frozen')},
    'unconverged_cases': [(c['seed'], c['t_c']) for c in valid if not all(c['converged'].values())],
    'per_case': valid,
}

# order probe and control worlds from evaluate.json
orders, variants = {}, {}
for s in seeds:
    p = os.path.join(root, f'seed{s}', 'evaluate.json')
    if not os.path.exists(p):
        continue
    e = json.load(open(p))
    for arm, o in e['order'].items():
        orders.setdefault(arm, []).append(o)
    for v, val in e['variants'].items():
        variants.setdefault(v, []).append(val)
for arm, os_ in orders.items():
    out['order'][arm] = {k: med([o[k] for o in os_ if o[k] is not None]) for k in ('d_order', 'kl_truth_abc', 'kl_truth_acb')}
    out['order'][arm]['n'] = len(os_)
for v, vals in variants.items():
    out['variants'][v] = {'d_hist_median': med([x['d_hist'] for x in vals]),
                          's_birth_minus_s_full_median': med([statistics.mean(x['s_birth']) - statistics.mean(x['s_full']) for x in vals if None not in x['s_birth'] and None not in x['s_full']]),
                          'n': len(vals)}

json.dump(out, open(os.path.join(root, 'secondary.json'), 'w'), indent=2, default=str)
print(json.dumps({k: v for k, v in out.items() if k != 'readapt'}, indent=1, default=str))
print('readapt:', json.dumps({k: v for k, v in out['readapt'].items() if k != 'per_case'}, indent=1, default=str))
