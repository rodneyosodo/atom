pub(crate) const CONSOLE_CSS: &str = r######"    :root {
      color-scheme: light;
      --bg: #f3f5f7;
      --panel: #ffffff;
      --soft: #f8fafc;
      --tint: #eef7f6;
      --border: #d7dee7;
      --border-strong: #b7c3d0;
      --text: #17202a;
      --muted: #607083;
      --accent: #0f766e;
      --accent-dark: #115e59;
      --accent-soft: #dff5f2;
      --warn: #936800;
      --danger: #b42318;
      --code: #101828;
      --shadow: 0 1px 2px rgba(16, 24, 40, .06);
    }

    * { box-sizing: border-box; }

    body {
      margin: 0;
      min-height: 100vh;
      background: var(--bg);
      color: var(--text);
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    button, input, select, textarea {
      font: inherit;
    }

    button {
      min-height: 36px;
      border: 1px solid var(--border);
      border-radius: 6px;
      background: var(--panel);
      color: var(--text);
      padding: 8px 10px;
      cursor: pointer;
    }

    button:hover { border-color: var(--border-strong); }

    button.primary {
      background: var(--accent);
      border-color: var(--accent);
      color: #fff;
      font-weight: 650;
    }

    button.primary:hover {
      background: var(--accent-dark);
      border-color: var(--accent-dark);
    }

    button.linkish {
      border-color: transparent;
      background: transparent;
      color: var(--accent-dark);
      padding-left: 0;
    }

    input, select, textarea {
      width: 100%;
      min-height: 36px;
      border: 1px solid var(--border);
      border-radius: 6px;
      background: #fff;
      color: var(--text);
      padding: 8px 9px;
    }

    textarea {
      min-height: 132px;
      resize: vertical;
      font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
      font-size: 13px;
      line-height: 1.45;
      tab-size: 2;
    }

    label {
      display: grid;
      gap: 5px;
      color: var(--muted);
      font-size: 12px;
    }

    label.check-row {
      display: flex;
      align-items: center;
      gap: 8px;
      color: var(--text);
      min-height: 36px;
    }

    label.check-row input {
      width: auto;
      min-height: 0;
    }

    h1, h2, h3, p { margin-top: 0; }

    h1 {
      font-size: 26px;
      line-height: 1.2;
      margin-bottom: 7px;
      letter-spacing: 0;
    }

    h2 {
      font-size: 17px;
      line-height: 1.3;
      margin-bottom: 10px;
      letter-spacing: 0;
    }

    h3 {
      font-size: 13px;
      line-height: 1.3;
      margin: 16px 0 8px;
      letter-spacing: 0;
      text-transform: uppercase;
      color: var(--muted);
    }

    p { line-height: 1.5; }

    code {
      font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
      background: var(--soft);
      border: 1px solid var(--border);
      border-radius: 4px;
      padding: 1px 4px;
    }

    pre {
      margin: 0;
      min-height: 112px;
      max-height: 360px;
      overflow: auto;
      white-space: pre-wrap;
      word-break: break-word;
      background: var(--soft);
      border: 1px solid var(--border);
      border-radius: 6px;
      color: var(--code);
      padding: 10px;
      font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
      font-size: 13px;
      line-height: 1.45;
    }

    details {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--soft);
      padding: 8px 10px;
      margin-top: 10px;
    }

    summary {
      cursor: pointer;
      color: var(--accent-dark);
      font-weight: 650;
      font-size: 13px;
    }

    details pre, details textarea { margin-top: 8px; }

    .app-shell {
      display: grid;
      grid-template-columns: 292px minmax(0, 1fr) 292px;
      min-height: 100vh;
    }

    .side-nav {
      background: var(--panel);
      border-right: 1px solid var(--border);
      min-width: 0;
    }

    .docs-panel {
      background: var(--panel);
      border-left: 1px solid var(--border);
      min-width: 0;
    }

    .side-scroll, .docs-scroll {
      height: 100vh;
      overflow: auto;
    }

    .brand {
      padding: 18px;
      border-bottom: 1px solid var(--border);
    }

    .brand h1 {
      font-size: 19px;
      margin-bottom: 4px;
    }

    .subtitle, .help, .muted {
      color: var(--muted);
      font-size: 13px;
      line-height: 1.45;
    }

    .nav-section {
      padding: 14px 16px;
      border-bottom: 1px solid var(--border);
    }

    .nav-list {
      display: grid;
      gap: 6px;
    }

    .nav-button {
      width: 100%;
      display: grid;
      gap: 2px;
      text-align: left;
      border-color: transparent;
      background: transparent;
      padding: 9px 10px;
    }

    .nav-button.active {
      background: var(--accent-soft);
      border-color: #a9ded8;
      color: var(--accent-dark);
    }

    .nav-button strong { font-size: 13px; }

    .nav-button span {
      color: var(--muted);
      font-size: 12px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .main {
      height: 100vh;
      min-width: 0;
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
    }

    .topbar {
      background: var(--panel);
      border-bottom: 1px solid var(--border);
      padding: 12px;
      display: grid;
      grid-template-columns: minmax(150px, 240px) minmax(240px, 1fr) auto auto;
      gap: 9px;
      align-items: end;
    }

    .workspace {
      height: 100%;
      min-width: 0;
      overflow: auto;
      padding: 16px;
    }

    .screen {
      display: none;
      max-width: 1160px;
      margin: 0 auto;
    }

    .screen.active { display: block; }

    .panel {
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      box-shadow: var(--shadow);
      padding: 15px;
      margin-bottom: 14px;
    }

    .panel.tint { background: var(--tint); }

    .grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }

    .grid-3 {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
    }

    .split {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      gap: 14px;
    }

    .task-grid {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
    }

    .task-card {
      display: grid;
      gap: 6px;
      min-height: 118px;
      text-align: left;
      padding: 14px;
      align-content: start;
    }

    .task-card strong { font-size: 15px; }

    .actions {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
      margin-top: 10px;
    }

    .status-row {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
    }

    .badge {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      max-width: 100%;
      border: 1px solid var(--border);
      border-radius: 999px;
      background: var(--soft);
      color: var(--muted);
      padding: 3px 8px;
      font-size: 12px;
      line-height: 1.25;
    }

    .badge.ok {
      background: var(--accent-soft);
      border-color: #b9e4df;
      color: var(--accent-dark);
    }

    .badge.warn {
      background: #fff7df;
      border-color: #ead49a;
      color: var(--warn);
    }

    .badge.error {
      background: #fff0ed;
      border-color: #f0b8ae;
      color: var(--danger);
    }

    .notice {
      border: 1px solid #e9d7a6;
      background: #fff9e9;
      color: #6b4d00;
      border-radius: 8px;
      padding: 10px;
      font-size: 13px;
      line-height: 1.45;
      margin-top: 10px;
    }

    .notice.danger {
      border-color: #f0b8ae;
      background: #fff0ed;
      color: var(--danger);
    }

    .wizard-nav {
      display: grid;
      grid-template-columns: repeat(5, minmax(0, 1fr));
      gap: 8px;
      margin-bottom: 12px;
    }

    .wizard-tab {
      text-align: left;
      min-height: 68px;
      padding: 10px;
    }

    .wizard-tab.active {
      background: var(--accent-soft);
      border-color: #a9ded8;
    }

    .wizard-step { display: none; }
    .wizard-step.active { display: block; }

    .endpoint-wizard-nav {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 8px;
      margin-bottom: 12px;
    }

    .endpoint-wizard-tab {
      display: grid;
      gap: 4px;
      min-height: 66px;
      text-align: left;
      padding: 10px;
    }

    .endpoint-wizard-tab.active {
      background: var(--accent-soft);
      border-color: #a9ded8;
      color: var(--accent-dark);
    }

    .endpoint-wizard-tab span {
      color: var(--muted);
      font-size: 12px;
    }

    .endpoint-wizard-step { display: none; }
    .endpoint-wizard-step.active { display: block; }

    .builder-sections {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
      gap: 10px;
    }

    .mini-panel {
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 12px;
      background: var(--soft);
    }

    .operation-list {
      display: grid;
      gap: 7px;
      max-height: 260px;
      overflow: auto;
      padding-right: 2px;
    }

    .operation-button {
      display: grid;
      gap: 4px;
      text-align: left;
      padding: 9px;
    }

    .operation-button strong { font-size: 13px; }

    .operation-button span {
      color: var(--muted);
      font-size: 12px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .template-groups {
      display: grid;
      gap: 12px;
      max-height: 520px;
      overflow: auto;
      padding-right: 2px;
    }

    .template-group {
      display: grid;
      gap: 6px;
    }

    .template-group h3 {
      margin-top: 0;
    }

    .template-button {
      width: 100%;
      display: grid;
      gap: 5px;
      text-align: left;
      background: var(--panel);
      padding: 9px;
    }

    .template-button.active {
      background: var(--accent-soft);
      border-color: #a9ded8;
      color: var(--accent-dark);
    }

    .template-button strong {
      font-size: 13px;
    }

    .template-button span {
      color: var(--muted);
      font-size: 12px;
      line-height: 1.35;
    }

    .template-choice-grid, .endpoint-list, .endpoint-logs, .mapping-rows {
      display: grid;
      gap: 10px;
      margin-top: 12px;
    }

    .template-choice-card, .endpoint-card, .log-row {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--panel);
      padding: 12px;
      display: grid;
      gap: 8px;
      min-width: 0;
    }

    .template-choice-card.active, .endpoint-card.active {
      border-color: #a9ded8;
      background: var(--accent-soft);
    }

    .endpoint-card-header {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 10px;
      align-items: start;
    }

    .endpoint-card-title {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
      min-width: 0;
    }

    .endpoint-meta {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 6px 12px;
      color: var(--muted);
      font-size: 12px;
    }

    .mapping-row {
      display: grid;
      grid-template-columns: minmax(130px, 1.2fr) auto minmax(120px, .8fr) minmax(150px, 1fr) auto;
      gap: 8px;
      align-items: end;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--soft);
      padding: 10px;
    }

    .mapping-arrow {
      color: var(--muted);
      font-size: 13px;
      padding-bottom: 10px;
      white-space: nowrap;
    }

    .log-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
      gap: 8px;
      color: var(--muted);
      font-size: 12px;
    }

    .schema-docs {
      display: grid;
      gap: 10px;
      font-size: 13px;
      line-height: 1.45;
    }

    .schema-docs dt {
      font-weight: 700;
      color: var(--text);
    }

    .schema-docs dd {
      margin: 2px 0 8px;
      color: var(--muted);
    }

    .type-list {
      max-height: 220px;
      overflow: auto;
      display: flex;
      gap: 5px;
      flex-wrap: wrap;
    }

    .pill {
      display: inline-flex;
      align-items: center;
      border: 1px solid var(--border);
      border-radius: 999px;
      padding: 3px 8px;
      font-size: 12px;
      color: var(--muted);
      background: var(--soft);
      margin: 1px;
    }

    .field-list {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 6px 10px;
      max-height: 180px;
      overflow: auto;
      padding: 9px;
      border: 1px solid var(--border);
      border-radius: 6px;
      background: var(--soft);
      margin-top: 10px;
    }

    .field-list label {
      display: flex;
      gap: 6px;
      align-items: center;
      color: var(--text);
      font-size: 13px;
    }

    .recipe-preview {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      gap: 12px;
    }

    .summary-box {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--soft);
      padding: 10px;
      min-height: 46px;
      color: var(--muted);
      font-size: 13px;
      line-height: 1.45;
      margin-top: 10px;
    }

    .hidden { display: none; }

    @media (max-width: 1040px) {
      .app-shell { grid-template-columns: 260px minmax(0, 1fr); }
      .docs-panel { display: none; }
      .task-grid, .builder-sections, .wizard-nav, .endpoint-wizard-nav { grid-template-columns: repeat(2, minmax(0, 1fr)); }
      .topbar { grid-template-columns: minmax(160px, 1fr) minmax(160px, 1fr); }
    }

    @media (max-width: 760px) {
      .app-shell { display: block; }
      .side-scroll, .docs-scroll, .main { height: auto; }
      .side-nav { border-right: 0; border-bottom: 1px solid var(--border); }
      .workspace { height: auto; }
      .grid, .grid-3, .split, .task-grid, .builder-sections, .wizard-nav, .endpoint-wizard-nav, .recipe-preview, .mapping-row, .endpoint-card-header { grid-template-columns: 1fr; }
      .mapping-arrow { padding-bottom: 0; }
    }
"######;
