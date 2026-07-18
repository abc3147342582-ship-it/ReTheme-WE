import { useEffect, useState, type FormEvent } from "react";
import {
  loginAccount,
  logoutAccount,
  redeemCdk,
  registerAccount,
  requestRegisterCode,
  startOAuthLogin,
  type OAuthProvider,
} from "./desktop-api";
import type { AccountStatus } from "./types";
import { useI18n } from "./i18n";

type Props = {
  open: boolean;
  status: AccountStatus;
  onClose: () => void;
  onChange: (status: AccountStatus) => void;
  notice?: string;
};

export default function AccountDialog({ open, status, onClose, onChange, notice = "" }: Props) {
  const { t } = useI18n();
  const [mode, setMode] = useState<"login" | "register">("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [cdk, setCdk] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (notice) setMessage(notice);
  }, [notice]);

  if (!open) return null;

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    try {
      const next = mode === "login"
        ? await loginAccount(email, password)
        : await registerAccount(email, code, password);
      onChange(next);
      setMessage(mode === "login" ? t("dialog.loginSuccess") : t("dialog.registerSuccess"));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function sendCode() {
    setBusy(true);
    setMessage("");
    try {
      const debugCode = await requestRegisterCode(email);
      setMessage(debugCode ? t("dialog.debugCode", { code: debugCode }) : t("dialog.codeSent"));
      if (debugCode) setCode(debugCode);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function redeem() {
    setBusy(true);
    setMessage("");
    try {
      onChange(await redeemCdk(cdk));
      setCdk("");
      setMessage(t("dialog.cdkSuccess"));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function oauth(provider: OAuthProvider) {
    setBusy(true);
    setMessage("");
    try {
      await startOAuthLogin(provider);
      setMessage(t("dialog.oauthOpened"));
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function logout() {
    setBusy(true);
    setMessage("");
    try {
      onChange(await logoutAccount());
      onClose();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="account-dialog-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="account-dialog" role="dialog" aria-modal="true" aria-labelledby="account-dialog-title">
        <button className="dialog-close" onClick={onClose} aria-label={t("dialog.close")}>×</button>
        {status.authenticated ? (
          <>
            <p className="eyebrow">{t("nav.account")}</p>
            <h2 id="account-dialog-title">{status.email}</h2>
            <p className="dialog-lead">{status.pro ? t("account.proSupporter") : t("account.free")} · {status.deviceName}</p>
            <div className="account-facts">
              <span><b>{t("dialog.device")}</b>{status.deviceName}</span>
              <span><b>{t("dialog.connection")}</b>{status.heartbeatState === "online" ? t("common.online") : status.heartbeatState === "grace" ? t("heartbeat.grace.title") : t("common.offline")}</span>
            </div>
            <label className="dialog-field"><span>{t("dialog.redeemCdk")}</span><div><input value={cdk} onChange={(event) => setCdk(event.target.value)} placeholder="RETHEME-XXXXX-XXXXX" /><button type="button" disabled={busy || !cdk.trim()} onClick={() => void redeem()}>{t("dialog.redeem")}</button></div></label>
            {message && <p className="dialog-message">{message}</p>}
            <button className="button secondary dialog-submit" disabled={busy} onClick={() => void logout()}>{t("dialog.logout")}</button>
          </>
        ) : (
          <>
            <p className="eyebrow">{t("nav.account")}</p>
            <h2 id="account-dialog-title">{mode === "login" ? t("dialog.loginTitle") : t("dialog.registerTitle")}</h2>
            <p className="dialog-lead">{t("dialog.lead")}</p>
            <div className="oauth-options">
              <button type="button" aria-label={t("dialog.github")} disabled={busy} onClick={() => void oauth("github")}><span>GH</span>{t("dialog.github")}</button>
              <button type="button" aria-label={t("dialog.linuxdo")} disabled={busy} onClick={() => void oauth("linuxdo")}><span>L</span>{t("dialog.linuxdo")}</button>
            </div>
            <div className="dialog-divider"><span>{t("dialog.emailAlternative")}</span></div>
            <div className="dialog-tabs"><button className={mode === "login" ? "is-active" : ""} onClick={() => setMode("login")}>{t("dialog.loginTab")}</button><button className={mode === "register" ? "is-active" : ""} onClick={() => setMode("register")}>{t("dialog.registerTab")}</button></div>
            <form onSubmit={(event) => void submit(event)}>
              <label className="dialog-field"><span>{t("dialog.email")}</span><input type="email" required value={email} onChange={(event) => setEmail(event.target.value)} placeholder="you@example.com" /></label>
              {mode === "register" && <label className="dialog-field"><span>{t("dialog.emailCode")}</span><div><input required value={code} onChange={(event) => setCode(event.target.value)} placeholder={t("dialog.sixDigitCode")} /><button type="button" disabled={busy || !email} onClick={() => void sendCode()}>{t("dialog.send")}</button></div></label>}
              <label className="dialog-field"><span>{t("dialog.password")}</span><input type="password" required minLength={8} value={password} onChange={(event) => setPassword(event.target.value)} placeholder={t("dialog.passwordHint")} /></label>
              {message && <p className="dialog-message">{message}</p>}
              <button className="button primary dialog-submit" disabled={busy}>{busy ? t("dialog.processing") : mode === "login" ? t("dialog.loginTab") : t("dialog.registerAndLogin")}</button>
            </form>
          </>
        )}
      </section>
    </div>
  );
}
