import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function AppSettings() {
  const [open, setOpen] = useState(false);
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(!import.meta.env.DEV);
  const [error, setError] = useState<string | null>(null);
  const developmentMode = import.meta.env.DEV;

  useEffect(() => {
    if (developmentMode) return;
    let active = true;
    invoke<boolean>("get_autostart_enabled")
      .then((value) => active && setEnabled(value))
      .catch((reason) => active && setError(String(reason)))
      .finally(() => active && setLoading(false));
    return () => {
      active = false;
    };
  }, [developmentMode]);

  async function changeAutostart(next: boolean) {
    setLoading(true);
    setError(null);
    try {
      const confirmed = await invoke<boolean>("set_autostart_enabled", { enabled: next });
      setEnabled(confirmed);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{ position: "fixed", right: 20, bottom: 20, zIndex: 1200 }}>
      {open && (
        <section
          aria-label="앱 설정"
          style={{
            width: 330,
            marginBottom: 10,
            padding: 18,
            borderRadius: 16,
            border: "1px solid rgba(148, 163, 184, 0.28)",
            background: "rgba(15, 23, 42, 0.98)",
            boxShadow: "0 18px 50px rgba(0, 0, 0, 0.34)",
            color: "#f8fafc",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "start" }}>
            <div>
              <strong style={{ display: "block", fontSize: 15 }}>앱 설정</strong>
              <span style={{ display: "block", marginTop: 4, color: "#94a3b8", fontSize: 12 }}>
                설치된 Foldering의 실행 방식을 설정합니다.
              </span>
            </div>
            <button
              type="button"
              aria-label="설정 닫기"
              onClick={() => setOpen(false)}
              style={iconButtonStyle}
            >
              ×
            </button>
          </div>

          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              marginTop: 18,
              padding: "14px 12px",
              borderRadius: 12,
              background: "rgba(30, 41, 59, 0.76)",
              cursor: developmentMode || loading ? "not-allowed" : "pointer",
            }}
          >
            <input
              type="checkbox"
              checked={enabled}
              disabled={developmentMode || loading}
              onChange={(event) => void changeAutostart(event.currentTarget.checked)}
              style={{ width: 18, height: 18 }}
            />
            <span>
              <strong style={{ display: "block", fontSize: 13 }}>시스템 시작 시 자동 실행</strong>
              <small style={{ display: "block", marginTop: 3, color: "#94a3b8", lineHeight: 1.45 }}>
                Windows 로그인 시 Foldering을 자동으로 실행합니다.
              </small>
            </span>
          </label>

          {developmentMode && (
            <p style={noteStyle}>
              개발 실행에서는 비활성화됩니다. 설치형 Foldering.exe에서 ON/OFF 할 수 있습니다.
            </p>
          )}
          {!developmentMode && loading && <p style={noteStyle}>설정 확인 중…</p>}
          {error && <p style={{ ...noteStyle, color: "#fda4af" }}>{error}</p>}
        </section>
      )}

      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        style={{
          minWidth: 108,
          height: 42,
          padding: "0 15px",
          border: "1px solid rgba(148, 163, 184, 0.3)",
          borderRadius: 999,
          background: "#111827",
          color: "#f8fafc",
          boxShadow: "0 10px 30px rgba(0, 0, 0, 0.25)",
          cursor: "pointer",
          fontWeight: 700,
        }}
      >
        ⚙ 앱 설정
      </button>
    </div>
  );
}

const iconButtonStyle = {
  width: 30,
  height: 30,
  borderRadius: 8,
  border: "1px solid rgba(148, 163, 184, 0.25)",
  background: "transparent",
  color: "#cbd5e1",
  cursor: "pointer",
  fontSize: 18,
} as const;

const noteStyle = {
  margin: "10px 2px 0",
  color: "#94a3b8",
  fontSize: 11,
  lineHeight: 1.5,
} as const;
