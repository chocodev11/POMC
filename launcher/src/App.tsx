import { useState, useCallback, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  HiHome,
  HiSquares2X2,
  HiNewspaper,
  HiCog6Tooth,
  HiMinus,
  HiSquare2Stack,
  HiXMark,
  HiPlay,
  HiChevronDown,
  HiArrowLeft,
  HiCube,
  HiUserPlus,
  HiTrash,
  HiArrowRightOnRectangle,
  HiPencil,
  HiFolder,
  HiPlus,
  HiDocumentDuplicate,
} from "react-icons/hi2";

type Page = "home" | "installations" | "news" | "settings";

interface AuthAccount {
  username: string;
  uuid: string;
  access_token: string;
  expires_at: number;
}

interface Installation {
  id: string;
  name: string;
  version: string;
  lastPlayed: string;
  directory: string;
  width: number;
  height: number;
}

interface GameVersion {
  id: string;
  version_type: string;
}

interface PatchNote {
  title: string;
  version: string;
  date: string;
  summary: string;
  image_url: string;
  entry_type: string;
  content_path: string;
}

const NAV_ITEMS: { id: Page; label: string; icon: React.ReactNode }[] = [
  { id: "home", label: "HOME", icon: <HiHome /> },
  { id: "installations", label: "INSTALLATIONS", icon: <HiSquares2X2 /> },
  { id: "news", label: "NEWS & UPDATES", icon: <HiNewspaper /> },
];

function App() {
  const [page, setPage] = useState<Page>("home");
  const [accounts, setAccounts] = useState<AuthAccount[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const [accountDropdownOpen, setAccountDropdownOpen] = useState(false);
  const [server] = useState("");
  const [keepOpen, setKeepOpen] = useState(true);
  const [installations, setInstallations] = useState<Installation[]>([
    {
      id: "default",
      name: "Latest Release",
      version: "1.21.11",
      lastPlayed: "Today",
      directory: "default",
      width: 854,
      height: 480,
    },
  ]);
  const [activeInstall, setActiveInstall] = useState("default");
  const [editingInstall, setEditingInstall] = useState<Installation | null>(null);
  const [dialogVersionOpen, setDialogVersionOpen] = useState(false);
  const selectedVersion =
    installations.find((i) => i.id === activeInstall)?.version || "1.21.11";
  const [versions, setVersions] = useState<GameVersion[]>([]);
  const [versionDropdownOpen, setVersionDropdownOpen] = useState(false);
  const [showSnapshots, setShowSnapshots] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [authLoading, setAuthLoading] = useState(false);
  const [status, setStatus] = useState("");
  const [news, setNews] = useState<PatchNote[]>([]);
  const [skinUrl, setSkinUrl] = useState<string | null>(null);

  const account = accounts[activeIndex] || null;
  const username = account?.username || "Steve";
  const [selectedNote, setSelectedNote] = useState<{
    title: string;
    body: string;
  } | null>(null);

  const appWindow = getCurrentWindow();

  const openNote = useCallback(async (note: PatchNote) => {
    try {
      const body = await invoke<string>("get_patch_content", {
        contentPath: note.content_path,
      });
      setSelectedNote({ title: note.title, body });
      setPage("news");
    } catch (e) {
      console.error("Failed to fetch content:", e);
    }
  }, []);

  const minimize = () => { appWindow.minimize(); };
  const toggleMaximize = () => { appWindow.toggleMaximize(); };
  const close = () => { appWindow.close(); };

  const loadSkin = useCallback((uuid: string) => {
    invoke<string>("get_skin_url", { uuid })
      .then(setSkinUrl)
      .catch(() => setSkinUrl(null));
  }, []);

  useEffect(() => {
    invoke<AuthAccount[]>("get_all_accounts").then((accs) => {
      if (accs.length > 0) {
        setAccounts(accs);
        setActiveIndex(0);
        loadSkin(accs[0].uuid);
      }
    });
    invoke<PatchNote[]>("get_patch_notes", { count: 6 })
      .then(setNews)
      .catch((e) => console.error("Failed to fetch news:", e));
    invoke<GameVersion[]>("get_versions", { showSnapshots: false })
      .then(setVersions)
      .catch((e) => console.error("Failed to fetch versions:", e));
  }, [loadSkin]);

  const addAccount = useCallback(async () => {
    setAuthLoading(true);
    setAccountDropdownOpen(false);
    setStatus("Opening browser — approve the sign-in...");
    try {
      const acc = await invoke<AuthAccount>("add_account");
      setAccounts((prev) => {
        const filtered = prev.filter((a) => a.uuid !== acc.uuid);
        return [...filtered, acc];
      });
      setActiveIndex(
        accounts.filter((a) => a.uuid !== acc.uuid).length
      );
      loadSkin(acc.uuid);
      setStatus(`Signed in as ${acc.username}`);
    } catch (e) {
      setStatus(`Auth failed: ${e}`);
    }
    setAuthLoading(false);
  }, [accounts, loadSkin]);

  const switchAccount = useCallback(
    (index: number) => {
      setActiveIndex(index);
      setAccountDropdownOpen(false);
      if (accounts[index]) {
        loadSkin(accounts[index].uuid);
      }
    },
    [accounts, loadSkin]
  );

  const removeAccount = useCallback(
    (uuid: string) => {
      invoke("remove_account", { uuid });
      setAccounts((prev) => prev.filter((a) => a.uuid !== uuid));
      setActiveIndex(0);
      setAccountDropdownOpen(false);
      setSkinUrl(null);
    },
    []
  );

  const handleLaunch = useCallback(async () => {
    setLaunching(true);
    setStatus("Checking assets...");
    try {
      await invoke("ensure_assets", { version: selectedVersion });
      setStatus("Launching POMC...");
      const result = await invoke<string>("launch_game", {
        username,
        server: server || null,
      });
      setStatus(result);
    } catch (e) {
      setStatus(`${e}`);
    }
    setTimeout(() => {
      setLaunching(false);
      setStatus("");
    }, 3000);
  }, [username, server, selectedVersion]);

  return (
    <div className="app">
      <div className="titlebar" data-tauri-drag-region>
        <div className="titlebar-left" data-tauri-drag-region>
          <span className="titlebar-icon"><HiCube /></span>
        </div>
        <span className="titlebar-title" data-tauri-drag-region>
          POMC Launcher
        </span>
        <div className="titlebar-controls">
          <button className="tb-btn" onClick={minimize}>
            <HiMinus />
          </button>
          <button className="tb-btn" onClick={toggleMaximize}>
            <HiSquare2Stack />
          </button>
          <button
            className="tb-btn tb-close"
            onClick={close}
          >
            <HiXMark />
          </button>
        </div>
      </div>

      <div className="layout">
        <nav className="sidebar">
          <div className="sidebar-brand">
            <div className="brand-icon"><HiCube /></div>
            <div className="brand-text">
              <span className="brand-name">POMC</span>
              <span className="brand-sub">LAUNCHER</span>
            </div>
            <span className="brand-version">v0.1.0</span>
          </div>

          <div className="sidebar-nav">
            {NAV_ITEMS.map((item) => (
              <button
                key={item.id}
                className={`nav-btn ${page === item.id ? "active" : ""}`}
                onClick={() => setPage(item.id)}
              >
                <span className="nav-icon">{item.icon}</span>
                <span className="nav-text">{item.label}</span>
              </button>
            ))}
          </div>

          <div className="sidebar-bottom">
            {account ? (
              <div className="account-switcher">
                {accountDropdownOpen && (
                  <div className="click-away" onClick={() => setAccountDropdownOpen(false)} />
                )}
                <button
                  className="account-bar"
                  onClick={() => setAccountDropdownOpen(!accountDropdownOpen)}
                >
                  <div
                    className="mc-head"
                    style={
                      skinUrl
                        ? { backgroundImage: `url(${skinUrl})` }
                        : undefined
                    }
                  />
                  <span className="account-username">{account.username}</span>
                  <HiChevronDown
                    className={`account-arrow ${accountDropdownOpen ? "open" : ""}`}
                  />
                </button>
                {accountDropdownOpen && (
                  <div className="account-dropdown-menu">
                    {accounts.map((acc, i) => (
                      <div
                        key={acc.uuid}
                        className={`account-option ${i === activeIndex ? "active" : ""}`}
                      >
                        <button
                          className="account-option-btn"
                          onClick={() => switchAccount(i)}
                        >
                          {acc.username}
                        </button>
                        <button
                          className="account-remove"
                          onClick={() => removeAccount(acc.uuid)}
                        >
                          <HiTrash />
                        </button>
                      </div>
                    ))}
                    <button
                      className="account-add"
                      onClick={addAccount}
                      disabled={authLoading}
                    >
                      <HiUserPlus />
                      <span>
                        {authLoading ? "Signing in..." : "Add account"}
                      </span>
                    </button>
                    <button
                      className="account-menu-btn"
                      onClick={() => {
                        setPage("settings");
                        setAccountDropdownOpen(false);
                      }}
                    >
                      <HiCog6Tooth />
                      <span>Settings</span>
                    </button>
                    <button
                      className="account-menu-btn logout"
                      onClick={() => {
                        if (account) removeAccount(account.uuid);
                      }}
                    >
                      <HiArrowRightOnRectangle />
                      <span>Log out</span>
                    </button>
                  </div>
                )}
              </div>
            ) : (
              <button
                className="sign-in-sidebar"
                onClick={addAccount}
                disabled={authLoading}
              >
                {authLoading ? "Signing in..." : "SIGN IN"}
              </button>
            )}
          </div>
        </nav>

        <main className="content">
          {page === "home" && (
            <div className="page home-page">
              <div className="hero-banner">
                <div className="hero-overlay" />
                <div className="hero-content">
                  <h1 className="hero-title">POMC</h1>
                  <p className="hero-subtitle">
                    RUST-NATIVE MINECRAFT CLIENT
                  </p>
                </div>
              </div>

              <div className="launch-bar">
                <button
                  className={`play-button ${launching ? "launching" : ""}`}
                  onClick={handleLaunch}
                  disabled={launching}
                >
                  <HiPlay className="play-icon" />
                  <span className="play-text">
                    {launching ? "LAUNCHING..." : "PLAY"}
                  </span>
                </button>
              </div>

              <div className="version-badge-wrapper">
                {versionDropdownOpen && (
                  <div className="click-away" onClick={() => setVersionDropdownOpen(false)} />
                )}
                <button
                  className="version-badge"
                  onClick={() => setVersionDropdownOpen(!versionDropdownOpen)}
                >
                  <HiCube className="version-badge-icon" />
                  <span>{selectedVersion}</span>
                  <HiChevronDown
                    className={`version-badge-arrow ${versionDropdownOpen ? "open" : ""}`}
                  />
                </button>
                {versionDropdownOpen && (
                  <div className="version-dropdown">
                    <div className="version-list">
                      {installations.map((inst) => (
                        <button
                          key={inst.id}
                          className={`version-item ${inst.id === activeInstall ? "active" : ""}`}
                          onClick={() => {
                            setActiveInstall(inst.id);
                            setVersionDropdownOpen(false);
                          }}
                        >
                          <span className="version-item-id">{inst.name}</span>
                          <span className="version-item-type">{inst.version}</span>
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>

              {status && <div className="status-toast">{status}</div>}

              <div className="news-section">
                <h2 className="news-heading">LATEST NEWS</h2>
                <div className="news-grid">
                  {news.slice(0, 3).map((item) => (
                    <div
                      className="news-card"
                      key={item.version}
                      onClick={() => openNote(item)}
                    >
                      <div
                        className="news-card-img"
                        style={{ backgroundImage: `url(${item.image_url})` }}
                      >
                        <span className="news-type-badge">
                          {item.entry_type}
                        </span>
                      </div>
                      <div className="news-card-body">
                        <span className="news-date">
                          {item.date.replace(/-/g, ".")}
                        </span>
                        <h3 className="news-title">{item.title}</h3>
                        <p className="news-desc">{item.summary}</p>
                      </div>
                    </div>
                  ))}
                  {news.length === 0 && (
                    <p className="news-loading">Loading patch notes...</p>
                  )}
                </div>
              </div>
            </div>
          )}

          {page === "installations" && (
            <div className="page installs-page">
              <div className="installs-header">
                <h2 className="installs-heading">INSTALLATIONS</h2>
                <button
                  className="installs-new-btn"
                  onClick={() =>
                    setEditingInstall({
                      id: "",
                      name: "",
                      version: "1.21.11",
                      lastPlayed: "Never",
                      directory: "",
                      width: 854,
                      height: 480,
                    })
                  }
                >
                  <HiPlus /> New Installation
                </button>
              </div>

              <div className="installs-list">
                {installations.map((inst) => (
                  <div
                    key={inst.id}
                    className={`install-card ${inst.id === activeInstall ? "active" : ""}`}
                  >
                    <div className="install-card-icon">
                      <HiCube />
                    </div>
                    <div className="install-card-info">
                      <span className="install-card-name">
                        {inst.name}
                      </span>
                      <span className="install-card-version">
                        {inst.version}
                      </span>
                    </div>
                    <span className="install-card-played">
                      {inst.lastPlayed}
                    </span>
                    <button
                      className="install-play-btn"
                      onClick={() => {
                        setActiveInstall(inst.id);
                        setPage("home");
                      }}
                    >
                      <HiPlay /> Play
                    </button>
                    <button
                      className="install-folder-btn"
                      onClick={() => console.log("Open:", inst.directory)}
                    >
                      <HiFolder />
                    </button>
                    <div className="install-card-actions">
                      <button
                        className="install-action-btn"
                        onClick={() => setEditingInstall({ ...inst })}
                        title="Edit"
                      >
                        <HiPencil />
                      </button>
                      <button
                        className="install-action-btn"
                        title="Duplicate"
                        onClick={() => {
                          const dup = {
                            ...inst,
                            id: Date.now().toString(36),
                            name: `${inst.name} (copy)`,
                          };
                          setInstallations((prev) => [...prev, dup]);
                        }}
                      >
                        <HiDocumentDuplicate />
                      </button>
                      {inst.id !== "default" && (
                        <button
                          className="install-action-btn delete"
                          title="Delete"
                          onClick={() => {
                            setInstallations((prev) =>
                              prev.filter((i) => i.id !== inst.id)
                            );
                            if (activeInstall === inst.id) {
                              setActiveInstall("default");
                            }
                          }}
                        >
                          <HiTrash />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {page === "news" && (
            <div className="page news-page">
              {selectedNote ? (
                <div className="note-viewer">
                  <button
                    className="note-back"
                    onClick={() => setSelectedNote(null)}
                  >
                    <HiArrowLeft /> Back
                  </button>
                  <h2 className="note-title">{selectedNote.title}</h2>
                  <div
                    className="note-body"
                    dangerouslySetInnerHTML={{ __html: selectedNote.body }}
                  />
                </div>
              ) : (
                <>
                  <h2 className="news-page-heading">NEWS & UPDATES</h2>
                  <div className="news-grid-full">
                    {news.map((item) => (
                      <div
                        className="news-card-wide"
                        key={item.version}
                        onClick={() => openNote(item)}
                      >
                        <div
                          className="news-card-img-wide"
                          style={{
                            backgroundImage: `url(${item.image_url})`,
                          }}
                        >
                          <span className="news-type-badge">
                            {item.entry_type}
                          </span>
                        </div>
                        <div className="news-card-body-wide">
                          <span className="news-date">
                            {item.date.replace(/-/g, ".")}
                          </span>
                          <h3 className="news-title">{item.title}</h3>
                          <p className="news-desc-full">{item.summary}</p>
                          <span className="news-version">{item.version}</span>
                        </div>
                      </div>
                    ))}
                    {news.length === 0 && (
                      <p className="news-loading">Loading patch notes...</p>
                    )}
                  </div>
                </>
              )}
            </div>
          )}

          {page === "settings" && (
            <div className="page settings-page">
              <h2 className="settings-heading">SETTINGS</h2>

              <div className="settings-section">
                <h3 className="settings-section-title">General</h3>

                <div className="settings-row">
                  <div className="settings-row-info">
                    <span className="settings-row-label">Language</span>
                    <span className="settings-row-desc">
                      Display language for the launcher
                    </span>
                  </div>
                  <div className="settings-row-control">
                    <button className="settings-select">
                      English
                    </button>
                  </div>
                </div>

                <div className="settings-row">
                  <div className="settings-row-info">
                    <span className="settings-row-label">
                      Keep launcher open
                    </span>
                    <span className="settings-row-desc">
                      Keep the launcher open after the game starts
                    </span>
                  </div>
                  <div className="settings-row-control">
                    <button
                      className={`settings-toggle ${keepOpen ? "on" : ""}`}
                      onClick={() => setKeepOpen(!keepOpen)}
                    >
                      <div className="settings-toggle-knob" />
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}
        </main>
      </div>

      {editingInstall && (
        <div className="dialog-overlay" onClick={() => { setEditingInstall(null); setDialogVersionOpen(false); }}>
          <div className="dialog" onClick={(e) => { e.stopPropagation(); if (dialogVersionOpen) setDialogVersionOpen(false); }}>
            <h2 className="dialog-title">
              {editingInstall.id ? "Edit Installation" : "New Installation"}
            </h2>

            <div className="dialog-fields">
              <div className="dialog-field">
                <label>NAME</label>
                <input
                  value={editingInstall.name}
                  onChange={(e) =>
                    setEditingInstall({ ...editingInstall, name: e.target.value })
                  }
                  placeholder="My Installation"
                  autoFocus
                />
              </div>
              <div className="dialog-field">
                <label>VERSION</label>
                <div className="custom-select-wrapper">
                  <button
                    className="custom-select"
                    onClick={() => setDialogVersionOpen(!dialogVersionOpen)}
                    type="button"
                  >
                    <span>{editingInstall.version}</span>
                    <HiChevronDown
                      className={`custom-select-arrow ${dialogVersionOpen ? "open" : ""}`}
                    />
                  </button>
                  {dialogVersionOpen && (
                    <div className="custom-select-dropdown" onClick={(e) => e.stopPropagation()}>
                      <label className="custom-select-toggle">
                        <input
                          type="checkbox"
                          checked={showSnapshots}
                          onChange={(e) => {
                            setShowSnapshots(e.target.checked);
                            invoke<GameVersion[]>("get_versions", {
                              showSnapshots: e.target.checked,
                            }).then(setVersions);
                          }}
                        />
                        <span>Show snapshots</span>
                      </label>
                      <div className="custom-select-list">
                        {versions.map((v) => (
                          <button
                            key={v.id}
                            className={`custom-select-item ${v.id === editingInstall.version ? "active" : ""}`}
                            onClick={() => {
                              setEditingInstall({
                                ...editingInstall,
                                version: v.id,
                              });
                              setDialogVersionOpen(false);
                            }}
                          >
                            <span>{v.id}</span>
                            {v.version_type !== "release" && (
                              <span className="custom-select-tag">
                                {v.version_type}
                              </span>
                            )}
                          </button>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
              <div className="dialog-field">
                <label>GAME DIRECTORY</label>
                <div className="dialog-browse">
                  <input
                    value={editingInstall.directory}
                    onChange={(e) =>
                      setEditingInstall({ ...editingInstall, directory: e.target.value })
                    }
                    placeholder="default"
                  />
                  <button
                    className="dialog-browse-btn"
                    onClick={async () => {
                      const path = await openDialog({ directory: true });
                      if (path) {
                        setEditingInstall({
                          ...editingInstall,
                          directory: path as string,
                        });
                      }
                    }}
                  >
                    <HiFolder />
                  </button>
                </div>
              </div>
              <div className="dialog-field">
                <label>RESOLUTION</label>
                <div className="dialog-resolution">
                  <input
                    type="number"
                    value={editingInstall.width}
                    onChange={(e) =>
                      setEditingInstall({
                        ...editingInstall,
                        width: parseInt(e.target.value) || 854,
                      })
                    }
                    placeholder="854"
                  />
                  <span className="dialog-resolution-x">×</span>
                  <input
                    type="number"
                    value={editingInstall.height}
                    onChange={(e) =>
                      setEditingInstall({
                        ...editingInstall,
                        height: parseInt(e.target.value) || 480,
                      })
                    }
                    placeholder="480"
                  />
                </div>
              </div>
            </div>

            <div className="dialog-actions">
              <button
                className="dialog-cancel"
                onClick={() => setEditingInstall(null)}
              >
                Cancel
              </button>
              <button
                className="dialog-save"
                onClick={async () => {
                  const isNew = !editingInstall.id;
                  const install = {
                    ...editingInstall,
                    id: editingInstall.id || Date.now().toString(36),
                    name: editingInstall.name || "Installation",
                    directory: editingInstall.directory || "default",
                  };
                  setInstallations((prev) => {
                    const filtered = prev.filter((i) => i.id !== install.id);
                    return [...filtered, install];
                  });
                  setEditingInstall(null);
                  if (isNew) {
                    setStatus(`Installing ${install.version}...`);
                    try {
                      await invoke("ensure_assets", { version: install.version });
                      setStatus(`${install.name} ready`);
                    } catch (e) {
                      setStatus(`Install failed: ${e}`);
                    }
                    setTimeout(() => setStatus(""), 3000);
                  }
                }}
              >
                {editingInstall.id ? "Save" : "Install"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
