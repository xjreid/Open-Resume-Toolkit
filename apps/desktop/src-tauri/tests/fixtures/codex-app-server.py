"""Deterministic local protocol fixture. Never contacts a provider."""
import json
import sys
import time

mode = sys.argv[1]
connected = False
quota_reads = 0
loaded_threads = 0

def emit(value):
    print(json.dumps(value), flush=True)

for line in sys.stdin:
    request = json.loads(line)
    method = request["method"]
    if "id" not in request:
        continue
    identifier = request["id"]
    result = {}
    if method == "config/read":
        result = {"config": {
            "cli_auth_credentials_store": "ephemeral", "web_search": "disabled",
            "features": {name: False for name in ["shell_tool", "unified_exec", "multi_agent", "plugins", "connectors", "computer_use", "image_generation", "view_image", "request_permissions_tool", "respect_system_proxy"]},
            "tools": {name: {"enabled": False} for name in ["update_plan", "experimental_request_user_input"]},
        }}
        result["layers"] = [{"config": result["config"], "disabledReason": None}]
    elif method == "model/list":
        result = {"data": [{"model": "gpt-6.1-sol", "hidden": False, "supportedReasoningEfforts": [{"reasoningEffort": "medium"}, {"reasoningEffort": "high"}]}]}
    elif method == "account/login/start":
        result = {"type": "chatgpt", "loginId": "login", "authUrl": "https://auth.openai.com/oauth/authorize"}
    elif method == "account/read":
        if mode == "slow_refresh" and request["params"].get("refreshToken"): time.sleep(0.5)
        result = {"account": {"type": "chatgpt", "planType": "plus"} if connected else None, "loadedThreads": loaded_threads}
    elif method == "account/logout":
        connected = False
    elif method == "account/rateLimits/read":
        if mode == "slow_quota": time.sleep(0.5)
        quota_reads += 1
        result = {"rateLimits": {"primary": {"usedPercent": 81 if mode == "falling_quota" and quota_reads > 1 else 80, "windowDurationMins": 300, "resetsAt": 2000000000}}}
        if mode == "missing_quota" or (mode == "quota_then_missing" and quota_reads > 1): result = {"rateLimits": {}}
        if mode == "quota_then_rejected" and quota_reads > 1:
            emit({"id": identifier, "error": {"message": "Synthetic quota failure"}})
            continue
    elif method == "thread/start":
        result = {"thread": {"id": "thread"}, "model": request["params"]["model"]}
        if mode != "reject_model": loaded_threads += 1
        if mode == "wrong_model": result["model"] = "unexpected"
    elif method == "thread/unsubscribe":
        if mode == "unsubscribe_failed":
            emit({"id": identifier, "error": {"message": "Synthetic unsubscribe failure"}})
            continue
        loaded_threads -= 1
        result = {"status": "unsubscribed"}
    elif method == "turn/start":
        if mode == "turn_start_rejected":
            emit({"id": identifier, "error": {"message": "Synthetic turn rejection"}})
            continue
        result = {"turn": {"id": "turn", "status": "inProgress"}}
    if mode == "reject_model" and method == "thread/start":
        emit({"id": identifier, "error": {"message": "This model is not available for your account"}})
        continue
    if mode == "wrong_id" and method == "account/read":
        identifier = 9999
    if mode == "malformed" and method == "account/read":
        print("bad json", flush=True)
        continue
    if mode == "timeout" and method == "account/read":
        time.sleep(3)
        continue
    emit({"id": identifier, "result": result})
    if mode == "stall_input" and method == "model/list": time.sleep(2)
    if mode == "stall_after_account" and method == "account/read": time.sleep(2)
    if method == "account/login/start":
        connected = mode != "declined"
        emit({"method": "account/login/completed", "params": {"loginId": "login", "success": connected, "error": "Sign-in declined by user" if not connected else None}})
    if method == "turn/start":
        if mode == "passive_status":
            for name in ["model/verification", "model/safetyBuffering/updated", "modelProvider/authRecoveryStarted", "modelProvider/authRecoveryCompleted", "warning", "configWarning", "deprecationNotice"]:
                emit({"method": name, "params": {"threadId": "thread", "turnId": "turn"}})
        if mode == "unknown_notification":
            emit({"method": "future/unknown", "params": {"private": "unlogged resume content"}})
            continue
        if mode == "tool":
            emit({"method": "item/started", "params": {"threadId": "thread", "turnId": "turn", "item": {"type": "commandExecution"}}})
            continue
        if mode == "permission":
            emit({"id": 8, "method": "item/permissions/requestApproval", "params": {}})
            continue
        if mode == "retry":
            emit({"method": "error", "params": {"threadId": "thread", "turnId": "turn", "willRetry": True, "error": {"message": "unlogged private fixture"}}})
        if mode != "missing_usage":
            usage = {"method": "thread/tokenUsage/updated", "params": {"threadId": "thread", "turnId": "turn", "tokenUsage": {"total": {"inputTokens": 100, "cachedInputTokens": 40, "outputTokens": 30, "reasoningOutputTokens": 20}, "last": {"inputTokens": 100, "cachedInputTokens": 40, "outputTokens": 30, "reasoningOutputTokens": 20}}}}
            emit(usage)
            emit(usage)
        emit({"method": "item/completed", "params": {"threadId": "thread", "turnId": "turn", "item": {"type": "agentMessage", "phase": "final_answer", "text": '{"schemaVersion":6}'}}})
        emit({"method": "turn/completed", "params": {"threadId": "thread", "turn": {"id": "turn", "status": "failed" if mode == "failed" else "completed"}}})
