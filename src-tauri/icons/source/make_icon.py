"""Draw Aximar's app icon: the Cayley graph of the alternating group A5.

A5 = <a, b | a^5 = b^2 = (ab)^3 = 1>.  Realise a as a 72 degree turn about
the z axis and b as a half turn about an axis in the xz-plane, tipped so
that ab has order 3.  The 60 rotations they generate, applied to one point
p, give the vertices; joining g p to g a p traces the 12 pentagons and
joining g p to g b p gives the 30 seams.  With p placed so that both kinds
of edge have the same length, the graph is the truncated icosahedron: a
football.

Writes master.svg (the full ball, back edges fading with depth) and
small.svg (the front half only, for icons of 64 px and under).  Plain
Python; no packages needed.
"""
import math
import os

INK, SEAM = "#1f2440", "#e0452b"
PAPER_TOP, PAPER_BOTTOM = "#fbf7ef", "#ece4d4"

FRAME = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient>
<filter id="sh" x="-10%" y="-10%" width="120%" height="130%"><feDropShadow dx="0" dy="12" stdDeviation="14" flood-color="#000" flood-opacity="0.28"/></filter>
</defs>
<rect x="100" y="100" width="824" height="824" rx="185" fill="url(#bg)" filter="url(#sh)"/>
{body}
</svg>
"""


def matmul(m, n):
    return [[sum(m[i][k] * n[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def apply(m, v):
    return [sum(m[i][k] * v[k] for k in range(3)) for i in range(3)]


def dot(u, v):
    return sum(x * y for x, y in zip(u, v))


def unit(v):
    n = math.sqrt(dot(v, v))
    return [x / n for x in v]


def cross(u, v):
    return [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]


def half_turn(phi):
    """The half turn about (sin phi, 0, cos phi): 2 u u^T - I"""
    u = [math.sin(phi), 0.0, math.cos(phi)]
    return [[2 * u[i] * u[j] - (i == j) for j in range(3)] for i in range(3)]


def trace(m):
    return m[0][0] + m[1][1] + m[2][2]


def cayley_graph():
    c, s = math.cos(2 * math.pi / 5), math.sin(2 * math.pi / 5)
    a = [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    # ab has order 3 when its trace is 0 (a turn by 120 degrees)
    lo, hi = 0.1, 1.5
    for _ in range(80):
        mid = (lo + hi) / 2
        if trace(matmul(a, half_turn(lo))) * trace(matmul(a, half_turn(mid))) <= 0:
            hi = mid
        else:
            lo = mid
    b = half_turn((lo + hi) / 2)

    # The truncated icosahedron's edge on the unit sphere.  |p - a p| fixes
    # p's angle from the z axis; p in the xz-plane makes the seams as long
    e = 4 / math.sqrt(58 + 18 * math.sqrt(5))
    alpha = math.asin(e / (2 * math.sin(math.pi / 5)))
    p = [math.sin(alpha), 0.0, math.cos(alpha)]

    def key(v):
        return tuple(round(x, 6) + 0.0 for x in v)

    ident = [[float(i == j) for j in range(3)] for i in range(3)]
    mats, seen, queue = [ident], {key(sum(ident, []))}, [ident]
    while queue:
        m = queue.pop(0)
        for g in (a, b):
            n = matmul(m, g)
            k = key(sum(n, []))
            if k not in seen:
                seen.add(k)
                mats.append(n)
                queue.append(n)
    pts = [apply(m, p) for m in mats]
    index = {key(q): i for i, q in enumerate(pts)}
    a_edges = [(i, index[key(apply(m, apply(a, p)))]) for i, m in enumerate(mats)]
    b_edges = sorted({tuple(sorted((i, index[key(apply(m, apply(b, p)))]))) for i, m in enumerate(mats)})
    assert (len(pts), len(a_edges), len(b_edges)) == (60, 60, 30)
    return pts, a_edges, b_edges


def draw(pts, a_edges, b_edges, scale=330, wa=30, wb=30, dotr=19, min_op=0.14, min_w=0.55, dots=True, cut=None):
    """The ball with vertex 0 facing the viewer and its seam pointing up.

    Edges fade and thin towards the back.  With `cut`, edges that reach
    further back than `cut` (on the unit sphere) are left out.
    """
    ez = unit(pts[0])
    j = next(y if x == 0 else x for x, y in b_edges if 0 in (x, y))
    w = pts[j]
    ey = unit([w[k] - dot(w, ez) * ez[k] for k in range(3)])
    ex = cross(ey, ez)
    P = [(dot(q, ex), dot(q, ey), dot(q, ez)) for q in pts]

    def proj(q):
        return 512 + scale * q[0], 512 - scale * q[1]

    def depth(z):
        # 0 at the back, 1 at the front, eased so the front half stays bold
        return max(0.0, min(1.0, (z + 1) / 2)) ** 1.6

    def lerp(lo, hi, t):
        return lo + (hi - lo) * t

    items = []
    for col, width, edges in ((INK, wa, a_edges), (SEAM, wb, b_edges)):
        for i, j in edges:
            items.append(((P[i][2] + P[j][2]) / 2, "edge", col, width, i, j))
    if dots:
        for i, q in enumerate(P):
            items.append((q[2] + 0.001, "dot", INK, dotr, i, None))
    # Back to front, so nearer edges cover further ones
    items.sort(key=lambda t: t[0])
    out = []
    for z, what, col, width, i, j in items:
        if cut is not None and what == "edge" and min(P[i][2], P[j][2]) < cut:
            continue
        d = depth(z)
        op, k = lerp(min_op, 1.0, d), lerp(min_w, 1.0, d)
        if what == "edge":
            (x1, y1), (x2, y2) = proj(P[i]), proj(P[j])
            out.append(
                f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{col}" '
                f'stroke-opacity="{op:.2f}" stroke-width="{width * k:.1f}" stroke-linecap="round"/>'
            )
        else:
            x, y = proj(P[i])
            out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{width * k:.1f}" fill="{col}" fill-opacity="{op:.2f}"/>')
    return FRAME.format(top=PAPER_TOP, bottom=PAPER_BOTTOM, body="\n".join(out))


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    graph = cayley_graph()
    with open(os.path.join(here, "master.svg"), "w") as f:
        f.write(draw(*graph))
    # Small icons: the front half only, with heavier strokes
    with open(os.path.join(here, "small.svg"), "w") as f:
        f.write(draw(*graph, wa=44, wb=44, dots=False, cut=-0.07, min_op=0.55, min_w=0.75))
