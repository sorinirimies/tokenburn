//! Stylesheet served at `/style.css`.

pub const CSS: &str = r#"
:root { color-scheme: dark; --bg:#14161f; --panel:#1c1f2b; --line:#2b3042; --fg:#d8dcea; --dim:#8b92aa; --accent:#7aa2f7; --ok:#9ece6a; --warn:#e0af68; --err:#f7768e; }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--fg); font:14px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif; }
main { max-width:1200px; margin:0 auto; padding:24px; display:flex; flex-direction:column; gap:20px; }
header.top { display:flex; flex-wrap:wrap; align-items:center; gap:16px; justify-content:space-between; }
h1 { margin:0; font-size:22px; color:var(--accent); }
h2 { margin:0 0 8px; font-size:14px; color:var(--dim); text-transform:uppercase; letter-spacing:.06em; }
form.filters { display:flex; flex-wrap:wrap; gap:10px; align-items:center; }
select, button { background:var(--panel); color:var(--fg); border:1px solid var(--line); border-radius:6px; padding:6px 10px; font:inherit; }
button { cursor:pointer; border-color:var(--accent); }
.cards { display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr)); gap:10px; }
.card { background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:12px; }
.card .label { color:var(--dim); font-size:12px; }
.card .value { font-size:22px; font-variant-numeric:tabular-nums; }
.card.cost .value { color:var(--ok); }
table { width:100%; border-collapse:collapse; background:var(--panel); border:1px solid var(--line); border-radius:8px; overflow:hidden; }
th, td { padding:8px 12px; text-align:right; font-variant-numeric:tabular-nums; border-bottom:1px solid var(--line); }
th { color:var(--accent); font-weight:600; background:#181b26; }
th:first-child, td:first-child, td.left, th.left { text-align:left; }
tr.total td { color:var(--warn); font-weight:700; }
td.cost { color:var(--ok); }
.bar { width:140px; height:8px; background:var(--line); border-radius:4px; overflow:hidden; display:inline-block; }
.bar > span { display:block; height:100%; background:var(--accent); }
.empty, .warn { padding:16px; background:var(--panel); border:1px solid var(--line); border-radius:8px; color:var(--dim); }
.warn { color:var(--warn); }
.badge { padding:2px 8px; border-radius:10px; border:1px solid var(--line); }
.badge.live { color:var(--ok); border-color:var(--ok); }
.filters { display:flex; flex-wrap:wrap; gap:10px; align-items:center; }
footer { color:var(--dim); font-size:12px; }
a { color:var(--accent); }
"#;
