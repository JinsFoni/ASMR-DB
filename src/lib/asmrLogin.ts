// asmr.one 登录 + 抓取 Token
//
// 流程：
// 1. 调用 Rust 命令 open_asmr_login_window 打开内嵌网页窗口（Rust 侧创建并带上
//    initialization_script，脚本在每次页面导航时原生执行，自动扫描 localStorage
//    里的 JWT 并写入 URL 片段，同时注入「获取 Token」按钮作为手动兜底）
// 2. 主窗口轮询 eval_in_window_result 在页面内读取 location.href / 直接扫
//    localStorage，拿到 JWT
// 3. 拿到后保存 token 并关闭登录窗口
import { invoke } from "@tauri-apps/api/core";

const LOGIN_LABEL = "asmr-login";
const TOKEN_MARK = "#ASMRTOKEN=";

/**
 * 在登录窗口内执行的扫描脚本：优先读 URL 片段里已写入的 token（初始化脚本写的），
 * 否则直接扫 localStorage 里的 JWT。返回 token 字符串或空串。
 * 页面侧 location.href 一定反映 replaceState，所以能可靠拿到片段。
 */
const SCAN_JS = `(function(){
  try {
    var h = location.href;
    var i = h.indexOf('${TOKEN_MARK}');
    if (i >= 0) { return decodeURIComponent(h.slice(i + ${TOKEN_MARK.length}).split('#')[0]); }
  } catch(e){}
  try {
    for (var j = 0; j < localStorage.length; j++) {
      var k = localStorage.key(j);
      var v = localStorage.getItem(k);
      if (!v || v.length < 30) continue;
      if (v.split('.').length === 3 && /^eyJ/.test(v)) return v;
      if (/token|jwt|auth/i.test(k) && v.length > 30) return v;
    }
  } catch(e){}
  return '';
})()`;

const POLL_INTERVAL_MS = 1500;
const TOTAL_TIMEOUT_MS = 5 * 60 * 1000;

/**
 * 打开 asmr.one 登录窗口，等待用户登录并自动/手动抓取 token。
 * 返回拿到的 token；超时或失败返回 null。
 * onStatus 回调用于把进度展示到界面上。
 */
export async function startAsmrLoginFlow(
  onStatus?: (msg: string) => void,
): Promise<string | null> {
  const status = (msg: string) => onStatus?.(msg);

  status("打开登录窗口…");
  try {
    // 加超时保护：即使 Rust 侧建窗卡住，前端也不无限等待
    await Promise.race([
      invoke("open_asmr_login_window"),
      new Promise((_, rej) => setTimeout(() => rej(new Error("打开登录窗口超时")), 6000)),
    ]);
  } catch (e) {
    console.error("open_asmr_login_window 失败", e);
    throw new Error(`无法打开 asmr.one 登录窗口：${e}`);
  }

  status("请在窗口中完成登录，将自动获取 Token…");
  const deadline = Date.now() + TOTAL_TIMEOUT_MS;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
    const res = await invoke<string | null>("eval_in_window_result", {
      label: LOGIN_LABEL,
      js: SCAN_JS,
    }).catch((e) => {
      // 页面加载中/窗口暂不可用属正常，等下一轮
      console.debug("eval_in_window_result 失败", e);
      return null;
    });
    if (res && res.trim()) {
      const token = res.trim();
      await invoke("set_asmr_token", { token }).catch((e) =>
        console.error("set_asmr_token 失败", e),
      );
      await invoke("close_asmr_login_window").catch(() => {});
      status("已获取 Token");
      return token;
    }
  }

  status("获取 Token 超时");
  await invoke("close_asmr_login_window").catch(() => {});
  return null;
}
