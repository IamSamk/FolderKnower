import hljs from "highlight.js/lib/core";
import go from "highlight.js/lib/languages/go";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import sql from "highlight.js/lib/languages/sql";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import yaml from "highlight.js/lib/languages/yaml";
import { useEffect, useMemo, useRef, useState } from "react";
import { readSource } from "../lib/backend";

hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("python", python);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("go", go);
hljs.registerLanguage("sql", sql);
hljs.registerLanguage("json", json);
hljs.registerLanguage("yaml", yaml);
hljs.registerLanguage("xml", xml);

const LINE = 20;

function langOf(path: string): string | undefined {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return (
    {
      ts: "typescript",
      tsx: "typescript",
      mts: "typescript",
      js: "javascript",
      jsx: "javascript",
      mjs: "javascript",
      py: "python",
      rs: "rust",
      go: "go",
      sql: "sql",
      json: "json",
      yml: "yaml",
      yaml: "yaml",
      prisma: "typescript",
    } as Record<string, string>
  )[ext];
}

export function CodeView({
  root,
  path,
  range,
}: {
  root: string;
  path: string;
  range?: [number, number];
}) {
  const [src, setSrc] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    setSrc(null);
    setErr(null);
    readSource(root, path)
      .then((s) => alive && setSrc(s))
      .catch((e) => alive && setErr(String(e)));
    return () => {
      alive = false;
    };
  }, [root, path]);

  const html = useMemo(() => {
    if (src == null) return "";
    const lang = langOf(path);
    try {
      return lang
        ? hljs.highlight(src, { language: lang, ignoreIllegals: true }).value
        : hljs.highlightAuto(src).value;
    } catch {
      return src.replace(
        /[&<>]/g,
        (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c] as string,
      );
    }
  }, [src, path]);

  const lines = src == null ? 0 : src.split("\n").length;

  useEffect(() => {
    if (!range || !box.current || src == null) return;
    const top = Math.max(0, (range[0] - 4) * LINE);
    box.current.scrollTo({ top, behavior: "smooth" });
  }, [range, src]);

  if (err)
    return (
      <div className="ak-muted p-4 text-sm">
        Cannot read {path}: {err}
      </div>
    );
  if (src == null) return <div className="ak-shimmer m-4 h-40 rounded-lg" />;
  return (
    <div ref={box} className="ak-code">
      <div className="ak-code-inner" style={{ height: lines * LINE + 24 }}>
        {range && (
          <div
            className="ak-code-hl"
            style={{ top: 12 + (range[0] - 1) * LINE, height: (range[1] - range[0] + 1) * LINE }}
          />
        )}
        <div className="ak-code-gutter">
          {Array.from({ length: lines }, (_, i) => (
            <div key={i}>{i + 1}</div>
          ))}
        </div>
        {/* biome-ignore lint/security/noDangerouslySetInnerHtml: highlight.js output of local source */}
        <pre className="ak-code-pre hljs" dangerouslySetInnerHTML={{ __html: html }} />
      </div>
    </div>
  );
}
