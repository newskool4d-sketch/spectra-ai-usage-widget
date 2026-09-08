import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { describePrepareFailure } from "../src/integrations/oauth-messages.ts";

describe("describePrepareFailure", () => {
  it("maps the disabled-feature stub to a preparing message", () => {
    const result = describePrepareFailure("native-oauth-disabled");
    assert.equal(result.status, "not-available");
    assert.equal(result.message, "네이티브 OAuth 연결은 준비 중입니다. 지금은 Codex App Server와 Claude Code 공식 도구로 사용량을 확인합니다.");
  });

  it("recognises the stub code inside an Error object", () => {
    const result = describePrepareFailure(new Error("invoke failed: native-oauth-disabled"));
    assert.ok(result.message.startsWith("네이티브 OAuth 연결은 준비 중입니다."));
  });

  it("keeps the generic failure message for other errors", () => {
    const result = describePrepareFailure(new Error("loopback bind failed"));
    assert.equal(result.status, "not-available");
    assert.equal(result.message, "이 기기에서 네이티브 OAuth 준비를 완료하지 못했습니다.");
  });
});
