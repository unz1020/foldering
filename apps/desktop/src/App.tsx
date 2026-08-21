import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CompositionEvent,
  type ChangeEvent,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ARCHIVE_FOLDER_NAME,
  MAX_MANAGED_DEPTH,
} from "@5level/core";
import {
  createLevel,
  resequenceLevels,
  validateSchema,
  type FolderSchema,
  type SchemaLevel,
} from "@5level/schema";

type Step = "workspace" | "schema" | "scan" | "rules" | "done";
type AppMode = "onboarding" | "review";

type ScanReport = {
  root: string;
  totalFolders: number;
  maxDepth: number;
  depthViolations: number;
  numberingViolations: number;
  duplicateNumberGroups: number;
  archiveMisuse: number;
  activeFolderLimitViolations: number;
  unreadableFolders: number;
};

type WorkspaceDraft = {
  name: string;
  root: string;
  watchDownloads: boolean;
  watchDesktop: boolean;
  schema: FolderSchema;
};

type DefaultWatchLocations = {
  downloads: string | null;
  desktop: string | null;
};

type FolderRecommendation = {
  path: string;
  relativePath: string;
  depth: number;
  score: number;
  managed: boolean;
  matchedLabels: string[];
};

type FileCandidate = {
  path: string;
  fileName: string;
  extension: string;
  sizeBytes: number;
  modifiedAtMs: number;
  sourceLabel: string;
  recommendations: FolderRecommendation[];
};

type FileTransaction = {
  id: string;
  kind: "MOVE" | "UNDO";
  source: string;
  destination: string;
  createdAtMs: number;
  originalTransactionId: string | null;
};

type TextInputProps = {
  value: string;
  onValueChange: (value: string) => void;
  placeholder?: string;
};

const defaultSchema: FolderSchema = {
  id: "custom-default",
  name: "내 폴더 구조",
  version: 1,
  levels: [
    {
      level: 1,
      key: "level_1",
      name: "첫 번째 분류",
      description: "업무 파일을 가장 먼저 구분하는 기준",
      examples: [],
    },
  ],
};

const initialWorkspace: WorkspaceDraft = {
  name: "내 업무",
  root: "",
  watchDownloads: true,
  watchDesktop: true,
  schema: defaultSchema,
};

const stepLabels: Array<{ id: Step; label: string }> = [
  { id: "workspace", label: "Workspace" },
  { id: "schema", label: "분류 순서" },
  { id: "scan", label: "구조 검사" },
  { id: "rules", label: "안전 규칙" },
  { id: "done", label: "완료" },
];

function ImeTextInput({ value, onValueChange, placeholder }: TextInputProps) {
  const [draft, setDraft] = useState(value);
  const composing = useRef(false);

  useEffect(() => {
    if (!composing.current) setDraft(value);
  }, [value]);

  function handleChange(event: ChangeEvent<HTMLInputElement>) {
    const next = event.currentTarget.value;
    setDraft(next);
    if (!composing.current) onValueChange(next);
  }

  function handleCompositionStart() {
    composing.current = true;
  }

  function handleCompositionEnd(event: CompositionEvent<HTMLInputElement>) {
    composing.current = false;
    const next = event.currentTarget.value;
    setDraft(next);
    onValueChange(next);
  }

  return (
    <input
      value={draft}
      placeholder={placeholder}
      onChange={handleChange}
      onCompositionStart={handleCompositionStart}
      onCompositionEnd={handleCompositionEnd}
      spellCheck={false}
    />
  );
}

function normalizeWorkspaceForSave(workspace: WorkspaceDraft): WorkspaceDraft {
  return {
    ...workspace,
    schema: {
      ...workspace.schema,
      levels: workspace.schema.levels.map((level) => ({
        ...level,
        examples: level.examples.map((value) => value.trim()).filter(Boolean),
      })),
    },
  };
}

function App() {
  const [mode, setMode] = useState<AppMode>("onboarding");
  const [hydrating, setHydrating] = useState(true);
  const [step, setStep] = useState<Step>("workspace");
  const [workspace, setWorkspace] = useState<WorkspaceDraft>(initialWorkspace);
  const [scan, setScan] = useState<ScanReport | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [loadingPreset, setLoadingPreset] = useState(false);

  useEffect(() => {
    let active = true;
    invoke<WorkspaceDraft | null>("load_workspace_config")
      .then((saved) => {
        if (!active || !saved) return;
        setWorkspace(saved);
        setMode("review");
      })
      .catch(() => {
        const legacy = localStorage.getItem("5level:onboarding-draft");
        if (!legacy || !active) return;
        try {
          setWorkspace(JSON.parse(legacy) as WorkspaceDraft);
        } catch {
          // Ignore invalid legacy draft.
        }
      })
      .finally(() => active && setHydrating(false));

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (mode !== "onboarding") return;
    localStorage.setItem("5level:onboarding-draft", JSON.stringify(workspace));
  }, [mode, workspace]);

  const schemaValidation = useMemo(
    () => validateSchema(workspace.schema),
    [workspace.schema],
  );
  const stepIndex = stepLabels.findIndex((item) => item.id === step);

  async function chooseRoot() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setWorkspace((current) => ({ ...current, root: selected }));
      setScan(null);
      setScanError(null);
    }
  }

  async function applyAdvertisingPreset() {
    setLoadingPreset(true);
    try {
      const response = await fetch("/advertising-agency.json");
      if (!response.ok) throw new Error("Preset을 불러오지 못했습니다.");
      const preset = (await response.json()) as { schema: FolderSchema };
      setWorkspace((current) => ({ ...current, schema: preset.schema }));
    } finally {
      setLoadingPreset(false);
    }
  }

  function updateLevel(index: number, patch: Partial<SchemaLevel>) {
    setWorkspace((current) => {
      const levels = current.schema.levels.map((item, itemIndex) =>
        itemIndex === index ? { ...item, ...patch } : item,
      );
      return {
        ...current,
        schema: { ...current.schema, levels: resequenceLevels(levels) },
      };
    });
  }

  function moveLevel(index: number, direction: -1 | 1) {
    setWorkspace((current) => {
      const target = index + direction;
      if (target < 0 || target >= current.schema.levels.length) return current;
      const levels = [...current.schema.levels];
      [levels[index], levels[target]] = [levels[target], levels[index]];
      return {
        ...current,
        schema: { ...current.schema, levels: resequenceLevels(levels) },
      };
    });
  }

  function addLevel() {
    setWorkspace((current) => {
      if (current.schema.levels.length >= MAX_MANAGED_DEPTH) return current;
      const next = (current.schema.levels.length + 1) as 1 | 2 | 3 | 4 | 5;
      return {
        ...current,
        schema: {
          ...current.schema,
          levels: [...current.schema.levels, createLevel(next)],
        },
      };
    });
  }

  function removeLevel(index: number) {
    setWorkspace((current) => {
      if (current.schema.levels.length <= 1) return current;
      const levels = current.schema.levels.filter((_, itemIndex) => itemIndex !== index);
      return {
        ...current,
        schema: { ...current.schema, levels: resequenceLevels(levels) },
      };
    });
  }

  async function runScan() {
    if (!workspace.root.trim()) {
      setScanError("먼저 Workspace 폴더를 선택하거나 경로를 입력하세요.");
      return;
    }

    setScanning(true);
    setScanError(null);
    try {
      const result = await invoke<ScanReport>("scan_folder", {
        rootPath: workspace.root,
      });
      setScan(result);
    } catch (error) {
      setScan(null);
      setScanError(String(error));
    } finally {
      setScanning(false);
    }
  }

  function next() {
    const order: Step[] = ["workspace", "schema", "scan", "rules", "done"];
    setStep(order[Math.min(order.indexOf(step) + 1, order.length - 1)]);
  }

  function back() {
    const order: Step[] = ["workspace", "schema", "scan", "rules", "done"];
    setStep(order[Math.max(order.indexOf(step) - 1, 0)]);
  }

  async function finishOnboarding() {
    const normalized = normalizeWorkspaceForSave(workspace);
    setWorkspace(normalized);
    await invoke("save_workspace_config", { config: normalized });
    localStorage.removeItem("5level:onboarding-draft");
    setMode("review");
  }

  const canContinue =
    step === "workspace"
      ? Boolean(
          workspace.name.trim() &&
            workspace.root.trim() &&
            (workspace.watchDownloads || workspace.watchDesktop),
        )
      : step === "schema"
        ? schemaValidation.valid
        : true;

  if (hydrating) {
    return (
      <div className="bootScreen">
        <div className="brandMark">5</div>
        <strong>5-Level</strong>
        <span>로컬 설정을 불러오는 중…</span>
      </div>
    );
  }

  if (mode === "review") {
    return (
      <ReviewMode
        workspace={workspace}
        onSettings={() => {
          setStep("workspace");
          setMode("onboarding");
        }}
      />
    );
  }

  return (
    <main className="shell">
      <aside className="sidebar">
        <Brand />
        <div className="stepList">
          {stepLabels.map((item, index) => {
            const active = item.id === step;
            const complete = index < stepIndex;
            return (
              <button
                className={`stepItem ${active ? "active" : ""} ${complete ? "complete" : ""}`}
                key={item.id}
                onClick={() => index <= stepIndex && setStep(item.id)}
                type="button"
              >
                <span>{complete ? "✓" : index + 1}</span>
                {item.label}
              </button>
            );
          })}
        </div>
        <div className="sidebarNote">
          <span className="statusDot" />
          Review Mode
          <small>관련 파일만 표시 · 이동은 승인 후 실행</small>
        </div>
      </aside>

      <section className="content">
        <header className="topbar">
          <div>
            <p className="eyebrow">ONBOARDING</p>
            <h1>{pageTitle(step)}</h1>
          </div>
          <span className="pill">Local only</span>
        </header>

        {step === "workspace" && (
          <div className="panel stack">
            <div className="introCopy">
              <h2>먼저 정리 기준이 되는 업무 공간을 지정하세요.</h2>
              <p>Workspace 자체는 5-Level 깊이에 포함되지 않습니다.</p>
            </div>

            <label className="field">
              <span>Workspace 이름</span>
              <ImeTextInput
                value={workspace.name}
                onValueChange={(name) =>
                  setWorkspace((current) => ({ ...current, name }))
                }
                placeholder="예: 내 업무"
              />
            </label>

            <label className="field">
              <span>Workspace 폴더</span>
              <div className="pathRow">
                <ImeTextInput
                  value={workspace.root}
                  onValueChange={(root) => {
                    setWorkspace((current) => ({ ...current, root }));
                    setScan(null);
                    setScanError(null);
                  }}
                  placeholder="폴더 경로를 입력하거나 선택하세요"
                />
                <button className="secondary" type="button" onClick={chooseRoot}>
                  폴더 선택
                </button>
              </div>
            </label>

            <div className="watchGrid">
              <label className="checkCard">
                <input
                  type="checkbox"
                  checked={workspace.watchDownloads}
                  onChange={(event) =>
                    setWorkspace((current) => ({
                      ...current,
                      watchDownloads: event.target.checked,
                    }))
                  }
                />
                <div>
                  <strong>Downloads</strong>
                  <span>관련성이 확인된 최상위 파일만 Review에 표시</span>
                </div>
              </label>
              <label className="checkCard">
                <input
                  type="checkbox"
                  checked={workspace.watchDesktop}
                  onChange={(event) =>
                    setWorkspace((current) => ({
                      ...current,
                      watchDesktop: event.target.checked,
                    }))
                  }
                />
                <div>
                  <strong>Desktop</strong>
                  <span>관련성이 확인된 최상위 파일만 Review에 표시</span>
                </div>
              </label>
            </div>
          </div>
        )}

        {step === "schema" && (
          <div className="stack">
            <div className="panel presetBar">
              <div>
                <strong>빠른 시작</strong>
                <span>광고대행사 기본 구조를 불러온 뒤 직접 수정할 수 있습니다.</span>
              </div>
              <button
                className="secondary"
                type="button"
                onClick={applyAdvertisingPreset}
                disabled={loadingPreset}
              >
                {loadingPreset ? "불러오는 중…" : "광고대행사 Preset"}
              </button>
            </div>

            <div className="levelList">
              {workspace.schema.levels.map((level, index) => (
                <div className="panel levelCard" key={level.key}>
                  <div className="levelHeader">
                    <div>
                      <span className="levelNumber">{index + 1}</span>
                      <strong>Level {index + 1}</strong>
                    </div>
                    <div className="levelActions">
                      <button type="button" onClick={() => moveLevel(index, -1)} disabled={index === 0}>↑</button>
                      <button type="button" onClick={() => moveLevel(index, 1)} disabled={index === workspace.schema.levels.length - 1}>↓</button>
                      <button type="button" onClick={() => removeLevel(index)} disabled={workspace.schema.levels.length === 1}>삭제</button>
                    </div>
                  </div>

                  <label className="field">
                    <span>분류 이름</span>
                    <ImeTextInput
                      value={level.name}
                      onValueChange={(name) => updateLevel(index, { name })}
                    />
                  </label>

                  <label className="field">
                    <span>설명</span>
                    <ImeTextInput
                      value={level.description}
                      onValueChange={(description) => updateLevel(index, { description })}
                    />
                  </label>

                  <label className="field">
                    <span>예시 (쉼표 구분)</span>
                    <ImeTextInput
                      value={level.examples.join(",")}
                      onValueChange={(raw) =>
                        updateLevel(index, { examples: raw.split(",") })
                      }
                      placeholder="예: 자코모, 교원웰스, 솔테라이브러리"
                    />
                  </label>
                </div>
              ))}
            </div>

            <button
              className="addLevel"
              type="button"
              onClick={addLevel}
              disabled={workspace.schema.levels.length >= MAX_MANAGED_DEPTH}
            >
              + Level 추가 ({workspace.schema.levels.length} / {MAX_MANAGED_DEPTH})
            </button>
            {!schemaValidation.valid && (
              <div className="errorBox">{schemaValidation.errors.join(" · ")}</div>
            )}
          </div>
        )}

        {step === "scan" && (
          <div className="panel stack">
            <div className="introCopy">
              <h2>기존 폴더를 먼저 읽기 전용으로 검사합니다.</h2>
              <p>이 단계에서는 폴더명·번호·위치를 절대 변경하지 않습니다.</p>
            </div>
            <button className="primary" type="button" onClick={runScan} disabled={scanning}>
              {scanning ? "검사 중…" : "폴더 건강검진 실행"}
            </button>
            {scanError && <div className="errorBox">{scanError}</div>}
            {scan && (
              <div className="metricGrid">
                <Metric label="전체 폴더" value={scan.totalFolders} />
                <Metric label="최대 Depth" value={scan.maxDepth} warn={scan.maxDepth > 5} />
                <Metric label="5-Level 초과" value={scan.depthViolations} warn={scan.depthViolations > 0} />
                <Metric label="넘버링 위반" value={scan.numberingViolations} warn={scan.numberingViolations > 0} />
                <Metric label="중복 번호 그룹" value={scan.duplicateNumberGroups} warn={scan.duplicateNumberGroups > 0} />
                <Metric label="99 오용" value={scan.archiveMisuse} warn={scan.archiveMisuse > 0} />
              </div>
            )}
          </div>
        )}

        {step === "rules" && (
          <div className="panel rulesPanel">
            <Rule title="최대 5-Level" description="Workspace 아래 관리 폴더는 최대 5단계입니다." />
            <Rule title="01~98 활성 폴더" description={`99는 ${ARCHIVE_FOLDER_NAME} 전용으로 예약합니다.`} />
            <Rule title="신규 폴더 승인 필수" description="현재 버전은 신규 폴더를 자동 생성하지 않습니다." />
            <Rule title="관련 파일만 Review" description="기존 Workspace 구조와 의미 있는 일치가 없는 Downloads/Desktop 파일은 표시하지 않습니다." />
            <Rule title="추천 ≠ 이동" description="Review Mode의 모든 파일 이동에는 명시적 승인이 필요합니다." />
            <Rule title="덮어쓰기 금지" description="같은 이름 파일이 목적지에 존재하면 이동하지 않습니다." />
            <Rule title="Undo" description="승인한 이동은 로컬 Transaction Log에 기록됩니다." />
          </div>
        )}

        {step === "done" && (
          <div className="panel donePanel">
            <div className="doneIcon">✓</div>
            <p className="eyebrow">READY</p>
            <h2>{workspace.name} 설정이 준비되었습니다.</h2>
            <p>
              Review Mode에는 Workspace 구조와 관련성이 확인된 파일만 표시되며,
              이동 전에는 직접 승인합니다.
            </p>
            <button className="primary" type="button" onClick={finishOnboarding}>
              Review Mode 시작
            </button>
          </div>
        )}

        <footer className="footerNav">
          <button className="secondary" type="button" onClick={back} disabled={step === "workspace"}>
            이전
          </button>
          {step !== "done" && (
            <button className="primary" type="button" onClick={next} disabled={!canContinue}>
              다음
            </button>
          )}
        </footer>
      </section>
    </main>
  );
}

function ReviewMode({ workspace, onSettings }: { workspace: WorkspaceDraft; onSettings: () => void }) {
  const [defaults, setDefaults] = useState<DefaultWatchLocations>({ downloads: null, desktop: null });
  const [candidates, setCandidates] = useState<FileCandidate[]>([]);
  const [transactions, setTransactions] = useState<FileTransaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [queueError, setQueueError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [movingPath, setMovingPath] = useState<string | null>(null);
  const [undoingId, setUndoingId] = useState<string | null>(null);
  const [watcherCount, setWatcherCount] = useState(0);
  const refreshTimer = useRef<number | null>(null);

  const watchLocations = useMemo(() => {
    const items: string[] = [];
    if (workspace.watchDownloads && defaults.downloads) items.push(defaults.downloads);
    if (workspace.watchDesktop && defaults.desktop) items.push(defaults.desktop);
    return Array.from(new Set(items));
  }, [defaults, workspace.watchDownloads, workspace.watchDesktop]);

  const refreshQueue = useCallback(async () => {
    if (!workspace.root || watchLocations.length === 0) {
      setCandidates([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    setQueueError(null);
    try {
      const queue = await invoke<FileCandidate[]>("get_review_queue", {
        workspaceRoot: workspace.root,
        watchLocations,
      });
      setCandidates(queue);
    } catch (error) {
      setQueueError(String(error));
    } finally {
      setLoading(false);
    }
  }, [watchLocations, workspace.root]);

  const refreshTransactions = useCallback(async () => {
    try {
      const items = await invoke<FileTransaction[]>("list_recent_transactions");
      setTransactions(items);
    } catch {
      setTransactions([]);
    }
  }, []);

  useEffect(() => {
    invoke<DefaultWatchLocations>("get_default_watch_locations")
      .then(setDefaults)
      .catch((error) => setQueueError(String(error)));
  }, []);

  useEffect(() => {
    void refreshQueue();
    void refreshTransactions();
  }, [refreshQueue, refreshTransactions]);

  useEffect(() => {
    if (watchLocations.length === 0) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;

    invoke<number>("start_watcher", { watchLocations })
      .then((count) => !disposed && setWatcherCount(count))
      .catch((error) => !disposed && setQueueError(String(error)));

    listen("watch-files-changed", () => {
      if (refreshTimer.current) window.clearTimeout(refreshTimer.current);
      refreshTimer.current = window.setTimeout(() => void refreshQueue(), 450);
    }).then((cleanup) => {
      if (disposed) cleanup();
      else unlisten = cleanup;
    });

    return () => {
      disposed = true;
      unlisten?.();
      if (refreshTimer.current) window.clearTimeout(refreshTimer.current);
      void invoke("stop_watcher");
    };
  }, [refreshQueue, watchLocations]);

  async function moveCandidate(candidate: FileCandidate, destinationDir: string) {
    setMovingPath(candidate.path);
    setNotice(null);
    try {
      await invoke<FileTransaction>("move_file_review", {
        workspaceRoot: workspace.root,
        watchLocations,
        sourcePath: candidate.path,
        destinationDir,
      });
      setNotice(`${candidate.fileName} 이동을 완료했습니다.`);
      await Promise.all([refreshQueue(), refreshTransactions()]);
    } catch (error) {
      setNotice(`이동하지 않았습니다: ${String(error)}`);
    } finally {
      setMovingPath(null);
    }
  }

  async function chooseDestination(candidate: FileCandidate) {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: workspace.root,
    });
    if (typeof selected === "string") await moveCandidate(candidate, selected);
  }

  async function undo(transaction: FileTransaction) {
    setUndoingId(transaction.id);
    setNotice(null);
    try {
      await invoke<FileTransaction>("undo_transaction", {
        transactionId: transaction.id,
      });
      setNotice("파일을 원래 위치로 되돌렸습니다.");
      await Promise.all([refreshQueue(), refreshTransactions()]);
    } catch (error) {
      setNotice(`Undo하지 않았습니다: ${String(error)}`);
    } finally {
      setUndoingId(null);
    }
  }

  const undoneIds = useMemo(
    () =>
      new Set(
        transactions
          .filter((item) => item.kind === "UNDO" && item.originalTransactionId)
          .map((item) => item.originalTransactionId as string),
      ),
    [transactions],
  );
  const recentMoves = transactions.filter((item) => item.kind === "MOVE").slice(0, 8);

  return (
    <main className="shell reviewShell">
      <aside className="sidebar">
        <Brand />
        <div className="reviewNav">
          <button className="reviewNavItem active" type="button"><span>⌁</span>확인 필요</button>
          <a className="reviewNavItem" href="#activity"><span>↶</span>활동 기록</a>
          <button className="reviewNavItem" type="button" onClick={onSettings}><span>⚙</span>설정</button>
        </div>
        <div className="sidebarNote">
          <span className="statusDot" />
          {watcherCount > 0 ? `${watcherCount}곳 감시 중` : "감시 준비 중"}
          <small>관련 파일만 · 최상위 파일만 · 로컬 처리</small>
        </div>
      </aside>

      <section className="content reviewContent">
        <header className="topbar reviewTopbar">
          <div>
            <p className="eyebrow">REVIEW MODE</p>
            <h1>{workspace.name}</h1>
            <p className="workspacePath">{workspace.root}</p>
          </div>
          <button
            className="secondary"
            type="button"
            onClick={() => void refreshQueue()}
            disabled={loading}
          >
            {loading ? "확인 중…" : "새로고침"}
          </button>
        </header>

        <div className="reviewMetrics">
          <div><span>승인 필요</span><strong>{candidates.length}</strong></div>
          <div><span>관련 파일</span><strong>{candidates.length}</strong></div>
          <div><span>감시 위치</span><strong>{watchLocations.length}</strong></div>
        </div>

        {notice && <div className="noticeBox">{notice}</div>}
        {queueError && <div className="errorBox">{queueError}</div>}

        <section className="reviewSection">
          <div className="sectionHeading">
            <div>
              <p className="eyebrow">RELATED INBOX</p>
              <h2>관련 파일 확인</h2>
            </div>
            <span className="pill">승인 전 이동 없음</span>
          </div>

          {loading ? (
            <div className="emptyState"><strong>관련 파일을 확인하고 있습니다…</strong></div>
          ) : candidates.length === 0 ? (
            <div className="emptyState successEmpty">
              <div className="emptyIcon">✓</div>
              <strong>현재 승인할 관련 파일이 없습니다.</strong>
              <span>Downloads/Desktop의 관련 없는 파일은 표시하지 않고 그대로 둡니다.</span>
            </div>
          ) : (
            <div className="candidateList">
              {candidates.map((candidate) => (
                <CandidateCard
                  key={candidate.path}
                  candidate={candidate}
                  moving={movingPath === candidate.path}
                  onApprove={(destination) => void moveCandidate(candidate, destination)}
                  onChoose={() => void chooseDestination(candidate)}
                />
              ))}
            </div>
          )}
        </section>

        <section className="reviewSection" id="activity">
          <div className="sectionHeading">
            <div>
              <p className="eyebrow">TRANSACTION LOG</p>
              <h2>최근 이동</h2>
            </div>
            <span className="pill">Undo</span>
          </div>

          {recentMoves.length === 0 ? (
            <div className="activityEmpty">아직 승인한 파일 이동이 없습니다.</div>
          ) : (
            <div className="activityList">
              {recentMoves.map((transaction) => {
                const undone = undoneIds.has(transaction.id);
                return (
                  <div className="activityRow" key={transaction.id}>
                    <div className="activityIcon">{undone ? "↶" : "→"}</div>
                    <div className="activityMain">
                      <strong>{fileNameFromPath(transaction.destination)}</strong>
                      <span>{shortPath(transaction.source)} → {shortPath(transaction.destination)}</span>
                    </div>
                    <div className="activityMeta">
                      <span>{formatDate(transaction.createdAtMs)}</span>
                      <button
                        className="secondary tinyButton"
                        type="button"
                        disabled={undone || undoingId === transaction.id}
                        onClick={() => void undo(transaction)}
                      >
                        {undone
                          ? "되돌림 완료"
                          : undoingId === transaction.id
                            ? "되돌리는 중…"
                            : "Undo"}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </section>
      </section>
    </main>
  );
}

function CandidateCard({
  candidate,
  moving,
  onApprove,
  onChoose,
}: {
  candidate: FileCandidate;
  moving: boolean;
  onApprove: (destination: string) => void;
  onChoose: () => void;
}) {
  const [selectedPath, setSelectedPath] = useState(
    candidate.recommendations[0]?.path ?? "",
  );
  const selected = candidate.recommendations.find(
    (item) => item.path === selectedPath,
  );

  useEffect(() => {
    setSelectedPath(candidate.recommendations[0]?.path ?? "");
  }, [candidate.path, candidate.recommendations]);

  return (
    <article className="candidateCard">
      <div className="fileGlyph">
        {candidate.extension ? candidate.extension.slice(0, 4).toUpperCase() : "FILE"}
      </div>
      <div className="candidateMain">
        <div className="fileTitleRow">
          <div>
            <strong>{candidate.fileName}</strong>
            <span>
              {candidate.sourceLabel} · {formatBytes(candidate.sizeBytes)} · {formatDate(candidate.modifiedAtMs)}
            </span>
          </div>
          <span className="sourceBadge">{candidate.sourceLabel}</span>
        </div>

        <div className="recommendationBox">
          <div className="recommendationTop">
            <span>추천 기존 폴더</span>
            {selected && <strong>{Math.round(selected.score * 100)}% 일치</strong>}
          </div>
          <select
            value={selectedPath}
            onChange={(event) => setSelectedPath(event.target.value)}
          >
            {candidate.recommendations.map((item, index) => (
              <option value={item.path} key={item.path}>
                {index + 1}순위 · {item.relativePath}
              </option>
            ))}
          </select>
          {selected && (
            <div className="recommendationMeta">
              <span>Level {selected.depth} / 5</span>
              <span>매칭: {selected.matchedLabels.join(", ")}</span>
              {!selected.managed && (
                <span className="legacyWarning">기존 규칙 위반 경로 · Review 승인만 가능</span>
              )}
            </div>
          )}
        </div>
      </div>
      <div className="candidateActions">
        <button
          className="primary"
          type="button"
          disabled={!selectedPath || moving}
          onClick={() => selectedPath && onApprove(selectedPath)}
        >
          {moving ? "이동 중…" : "이동 승인"}
        </button>
        <button className="secondary" type="button" disabled={moving} onClick={onChoose}>
          다른 폴더
        </button>
      </div>
    </article>
  );
}

function Brand() {
  return (
    <div className="brand">
      <div className="brandMark">5</div>
      <div>
        <strong>5-Level</strong>
        <span>Local Folder System</span>
      </div>
    </div>
  );
}

function pageTitle(step: Step) {
  switch (step) {
    case "workspace": return "업무 공간 만들기";
    case "schema": return "폴더링 순서 정의";
    case "scan": return "기존 폴더 건강검진";
    case "rules": return "5-Level 안전 규칙";
    case "done": return "설정 완료";
  }
}

function Metric({ label, value, warn = false }: { label: string; value: number; warn?: boolean }) {
  return (
    <div className={`metric ${warn ? "warn" : ""}`}>
      <span>{label}</span>
      <strong>{value.toLocaleString()}</strong>
    </div>
  );
}

function Rule({ title, description }: { title: string; description: string }) {
  return (
    <div className="ruleItem">
      <span>✓</span>
      <div>
        <strong>{title}</strong>
        <p>{description}</p>
      </div>
    </div>
  );
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

function formatDate(value: number) {
  if (!value) return "시간 정보 없음";
  return new Intl.DateTimeFormat("ko-KR", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function fileNameFromPath(path: string) {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

function shortPath(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length <= 3 ? parts.join("/") : `…/${parts.slice(-3).join("/")}`;
}

export default App;
