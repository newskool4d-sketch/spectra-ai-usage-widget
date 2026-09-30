import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { fileURLToPath } from "node:url";
import { createElement, isValidElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { transformSync } from "rolldown/utils";

// Reuse the already installed Vite compiler to execute the real TSX component.
// These verify SSR output and callback wiring, not browser or native installation.
const tsx = registerHooks({
  load(url, context, nextLoad) {
    if (url.endsWith("/UpdateNotice.tsx")) {
      const filename = fileURLToPath(url);
      const result = transformSync(filename, readFileSync(filename, "utf8"), { jsx: { runtime: "automatic" } });
      assert.deepEqual(result.errors, []);
      return { format: "module", source: result.code, shortCircuit: true };
    }
    return nextLoad(url, context);
  },
});
const { UpdateNotice } = await import("../src/components/UpdateNotice.tsx");
tsx.deregister();

const props = (phase: string, message = "업데이트 상태", version: string | null = "0.2.7") => ({
  state: { phase, version, message }, visible: true,
  onInstall: () => {}, onRetry: () => {}, onDismiss: () => {},
});
const render = (value: ReturnType<typeof props>) => renderToStaticMarkup(createElement(UpdateNotice, value));

function buttons(node: ReactNode): Array<Record<string, any>> {
  if (Array.isArray(node)) return node.flatMap(buttons);
  if (!isValidElement(node)) return [];
  const elementProps = node.props as Record<string, any>;
  return node.type === "button" ? [elementProps] : buttons(elementProps.children);
}

describe("global update notice recovery", () => {
  it("keeps an initial background check and a dismissed notice quiet", () => {
    assert.equal(render({ ...props("checking"), visible: false }), "");
    assert.equal(render({ ...props("available"), visible: false }), "");
  });

  it("preserves the error message and retry action after install or check failure", () => {
    for (const version of ["0.2.7", null]) {
      const html = render(props("error", "업데이트 실패: 연결 제한 시간 초과", version));
      assert.match(html, /role="alert"/);
      assert.match(html, /연결 제한 시간 초과/);
      assert.match(html, /다시 확인/);
      assert.doesNotMatch(html, /지금 설치/);
    }
  });

  it("routes failure retry to the check callback rather than installing", () => {
    const calls: string[] = [];
    const value = { ...props("error"), onRetry: () => calls.push("check"), onInstall: () => calls.push("install") };
    const action = buttons(UpdateNotice(value)).find(button => button.children === "다시 확인");
    assert.ok(action, "an error must expose the retry control");
    assert.equal(action.disabled, false);
    action.onClick();
    assert.deepEqual(calls, ["check"]);
  });

  it("retains progress during retry checks and installation with both controls disabled", () => {
    for (const phase of ["checking", "installing"]) {
      const value = props(phase, "서버 응답을 기다리고 있습니다.");
      const html = render(value);
      assert.match(html, /서버 응답을 기다리고 있습니다/);
      const controls = buttons(UpdateNotice(value));
      assert.equal(controls.length, 2);
      assert.ok(controls.every(button => button.disabled === true));
      assert.match(html, phase === "checking" ? /확인 중/ : /설치 중/);
    }
  });

  it("restores install after a successful retry and supports dismissal", () => {
    const calls: string[] = [];
    const value = { ...props("available"), onInstall: () => calls.push("install"), onDismiss: () => calls.push("dismiss") };
    assert.match(render(value), /새 버전 0\.2\.7/);
    const controls = buttons(UpdateNotice(value));
    assert.ok(controls.every(button => !button.disabled));
    controls.find(button => button.children === "지금 설치")!.onClick();
    controls.find(button => button.children === "나중에")!.onClick();
    assert.deepEqual(calls, ["install", "dismiss"]);
  });

  it("clears the notice after a retry finds no update", () => {
    for (const phase of ["up-to-date", "idle", "unsupported"]) {
      assert.equal(render(props(phase)), "");
    }
  });
});
