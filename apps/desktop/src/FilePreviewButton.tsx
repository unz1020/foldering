import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type FilePreview = {
  kind: "image" | "pdf" | "text" | "unsupported";
  mimeType: string | null;
  dataUrl: string | null;
  text: string | null;
  message: string | null;
};

type Props = {
  filePath: string;
  fileName: string;
  watchLocations: string[];
};

export default function FilePreviewButton({ filePath, fileName, watchLocations }: Props) {
  const [visible, setVisible] = useState(false);
  const [loading, setLoading] = useState(false);
  const [preview, setPreview] = useState<FilePreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);

  async function showPreview() {
    setVisible(true);
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FilePreview>("get_file_preview", {
        filePath,
        watchLocations,
      });
      setPreview(result);
    } catch (previewError) {
      setPreview(null);
      setError(String(previewError));
    } finally {
      setLoading(false);
    }
  }

  async function openOriginal() {
    setOpening(true);
    setError(null);
    try {
      await invoke("open_candidate_file", { filePath, watchLocations });
    } catch (openError) {
      setError(String(openError));
    } finally {
      setOpening(false);
    }
  }

  return (
    <>
      <button className="secondary" type="button" onClick={() => void showPreview()}>
        미리보기
      </button>

      {visible && (
        <div className="previewOverlay" role="presentation" onMouseDown={() => setVisible(false)}>
          <section
            className="previewModal"
            role="dialog"
            aria-modal="true"
            aria-label={`${fileName} 미리보기`}
            onMouseDown={(event) => event.stopPropagation()}
          >
            <header className="previewHeader">
              <div>
                <span>FILE PREVIEW</span>
                <strong>{fileName}</strong>
              </div>
              <button className="previewClose" type="button" onClick={() => setVisible(false)}>
                ×
              </button>
            </header>

            <div className="previewBody">
              {loading && <div className="previewEmpty">미리보기를 불러오는 중…</div>}
              {!loading && error && <div className="errorBox">{error}</div>}
              {!loading && !error && preview?.kind === "image" && preview.dataUrl && (
                <img className="previewImage" src={preview.dataUrl} alt={fileName} />
              )}
              {!loading && !error && preview?.kind === "pdf" && preview.dataUrl && (
                <iframe className="previewPdf" src={preview.dataUrl} title={`${fileName} PDF 미리보기`} />
              )}
              {!loading && !error && preview?.kind === "text" && (
                <pre className="previewText">{preview.text}</pre>
              )}
              {!loading && !error && preview?.kind === "unsupported" && (
                <div className="previewEmpty">
                  <strong>앱 내부 미리보기 미지원</strong>
                  <span>{preview.message}</span>
                </div>
              )}
            </div>

            <footer className="previewFooter">
              <span>파일은 이동하지 않고 읽기만 합니다.</span>
              <div>
                <button className="secondary" type="button" onClick={() => setVisible(false)}>
                  닫기
                </button>
                <button className="primary" type="button" disabled={opening} onClick={() => void openOriginal()}>
                  {opening ? "여는 중…" : "원본 열기"}
                </button>
              </div>
            </footer>
          </section>
        </div>
      )}
    </>
  );
}
