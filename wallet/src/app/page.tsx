'use client';

import { useEffect, useState, useCallback } from 'react';
import { generateKeypair, pubkeyFromPrivate, buildTransferTx } from '@/lib/crypto';
import {
  getBalance, getRecentBlockhash, sendTransaction, isNodeAlive,
  getBlockHeight, getTxHistory, listTokens, getTokenBalance,
  TxHistoryEntry, TokenInfo,
} from '@/lib/rpc';

const STORAGE_KEY = 'edda_wallet_v1';
interface WalletState { privateKey: string; publicKey: string; }

function shortAddr(a: string) { return a.slice(0, 6) + '···' + a.slice(-6); }
function shortSig(s: string)  { return s.slice(0, 12) + '…'; }
function fmtEdda(l: number)   { return (l / 1_000_000_000).toLocaleString('en', { maximumFractionDigits: 4 }); }

export default function Wallet() {
  const [wallet,     setWallet]     = useState<WalletState | null>(null);
  const [balance,    setBalance]    = useState<bigint>(0n);
  const [blockH,     setBlockH]     = useState(0);
  const [online,     setOnline]     = useState(false);
  const [tab,        setTab]        = useState<'send' | 'history' | 'tokens'>('send');
  const [txHistory,  setTxHistory]  = useState<TxHistoryEntry[]>([]);
  const [tokens,     setTokens]     = useState<TokenInfo[]>([]);
  const [tokenBals,  setTokenBals]  = useState<Record<string, number>>({});
  const [copied,     setCopied]     = useState(false);

  const [toAddr,   setToAddr]   = useState('');
  const [amount,   setAmount]   = useState('');
  const [sending,  setSending]  = useState(false);
  const [sendMsg,  setSendMsg]  = useState<{ type: 'ok'|'err'; text: string; sig?: string } | null>(null);

  const [tokenSendId,  setTokenSendId]  = useState('');
  const [tokenToAddr,  setTokenToAddr]  = useState('');
  const [tokenAmount,  setTokenAmount]  = useState('');
  const [tokenSending, setTokenSending] = useState(false);
  const [tokenSendMsg, setTokenSendMsg] = useState<{ type: 'ok'|'err'; text: string } | null>(null);

  const [importKey, setImportKey] = useState('');
  const [importErr, setImportErr] = useState('');
  const [showSetup, setShowSetup] = useState(false);

  useEffect(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) { try { setWallet(JSON.parse(saved)); } catch { localStorage.removeItem(STORAGE_KEY); } }
  }, []);

  const poll = useCallback(async () => {
    const alive = await isNodeAlive();
    setOnline(alive);
    if (!alive || !wallet) return;
    try {
      const [bal, height, hist, toks] = await Promise.all([
        getBalance(wallet.publicKey), getBlockHeight(),
        getTxHistory(wallet.publicKey, 30), listTokens(),
      ]);
      setBalance(bal); setBlockH(height); setTxHistory(hist); setTokens(toks);
      if (toks.length > 0) {
        const bals = await Promise.all(
          toks.map(t => getTokenBalance(t.id, wallet.publicKey).then(b => [t.id, b] as [string, number]))
        );
        setTokenBals(Object.fromEntries(bals));
      }
    } catch {}
  }, [wallet]);

  useEffect(() => { poll(); const id = setInterval(poll, 2000); return () => clearInterval(id); }, [poll]);

  function createWallet() {
    const kp = generateKeypair();
    localStorage.setItem(STORAGE_KEY, JSON.stringify(kp));
    setWallet(kp); setShowSetup(false);
  }

  function importWallet() {
    const key = importKey.trim().toLowerCase();
    if (!/^[0-9a-f]{64}$/.test(key)) { setImportErr('Must be 64 hex characters'); return; }
    try {
      const publicKey = pubkeyFromPrivate(key);
      const kp = { privateKey: key, publicKey };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(kp));
      setWallet(kp); setImportKey(''); setImportErr(''); setShowSetup(false);
    } catch { setImportErr('Invalid private key'); }
  }

  function copyAddress() {
    if (!wallet) return;
    navigator.clipboard.writeText(wallet.publicKey);
    setCopied(true); setTimeout(() => setCopied(false), 1500);
  }

  async function doSend() {
    if (!wallet) return;
    setSendMsg(null);
    const lamports = BigInt(Math.round(parseFloat(amount) * 1_000_000_000));
    if (lamports <= 0n) { setSendMsg({ type: 'err', text: 'Enter an amount' }); return; }
    if (!/^[0-9a-f]{64}$/.test(toAddr.trim())) { setSendMsg({ type: 'err', text: 'Invalid address' }); return; }
    if (lamports + 5_000n > balance) { setSendMsg({ type: 'err', text: 'Insufficient balance' }); return; }
    setSending(true);
    try {
      const blockhash = await getRecentBlockhash();
      const b64 = buildTransferTx(wallet.privateKey, toAddr.trim(), lamports, blockhash);
      const sig = await sendTransaction(b64);
      setSendMsg({ type: 'ok', text: 'Sent', sig });
      setToAddr(''); setAmount('');
      setTimeout(poll, 800);
    } catch (e: unknown) {
      setSendMsg({ type: 'err', text: e instanceof Error ? e.message : 'Failed' });
    } finally { setSending(false); }
  }

  async function doSendToken() {
    if (!wallet || !tokenSendId) return;
    setTokenSendMsg(null);
    const tok = tokens.find(t => t.id === tokenSendId);
    if (!tok) return;
    const rawAmt = Math.round(parseFloat(tokenAmount) * Math.pow(10, tok.decimals));
    if (!rawAmt || rawAmt <= 0) { setTokenSendMsg({ type: 'err', text: 'Enter an amount' }); return; }
    if (!/^[0-9a-f]{64}$/.test(tokenToAddr.trim())) { setTokenSendMsg({ type: 'err', text: 'Invalid address' }); return; }
    if (rawAmt > (tokenBals[tokenSendId] ?? 0)) { setTokenSendMsg({ type: 'err', text: 'Insufficient balance' }); return; }
    setTokenSending(true);
    try {
      const res = await fetch('/api/rpc', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'transferToken',
          params: [tokenSendId, wallet.publicKey, tokenToAddr.trim(), rawAmt] }),
      });
      const json = await res.json();
      if (json.error) throw new Error(json.error.message);
      setTokenSendMsg({ type: 'ok', text: `Sent ${tokenAmount} ${tok.symbol}` });
      setTokenToAddr(''); setTokenAmount('');
      setTimeout(poll, 800);
    } catch (e: unknown) {
      setTokenSendMsg({ type: 'err', text: e instanceof Error ? e.message : 'Failed' });
    } finally { setTokenSending(false); }
  }

  function deleteWallet() {
    if (!confirm('Delete wallet? Back up your private key first.')) return;
    localStorage.removeItem(STORAGE_KEY);
    setWallet(null); setBalance(0n); setTxHistory([]);
  }

  // ── Setup ────────────────────────────────────────────────────────────────────
  if (!wallet || showSetup) {
    return (
      <>
        <div className="card">
          <div className="header">
            <div className="logo"><span className="dot" />Edda</div>
            <span className="net-badge">Testnet</span>
          </div>
          <div className="setup">
            <h2>Your Edda Wallet</h2>
            <p>A fast, minimal wallet for the Edda Network.<br />Your keys stay in this browser.</p>
            <button className="btn btn-primary" onClick={createWallet}>Create Wallet</button>
            <div className="divider">or import existing</div>
            <input className="inp inp-mono" placeholder="Private key (64 hex characters)"
              value={importKey} onChange={e => { setImportKey(e.target.value); setImportErr(''); }}
              type="password" />
            {importErr && <div className="toast toast-err" style={{ marginTop: 10 }}>{importErr}</div>}
            <button className="btn btn-secondary" onClick={importWallet}>Import</button>
          </div>
        </div>
        <div className="footer">Edda Network · eddachain.com</div>
      </>
    );
  }

  const eddaBal  = Number(balance) / 1_000_000_000;
  const maxSend  = Math.max(0, (Number(balance) - 5_000) / 1_000_000_000);
  const selTok   = tokens.find(t => t.id === tokenSendId);

  return (
    <>
      <div className="card">
        {/* Header */}
        <div className="header">
          <div className="logo"><span className="dot" />Edda</div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <span className="status-line">
              <span className={`status-dot ${online ? 'online' : 'offline'}`} />
              {online ? `Block ${blockH.toLocaleString()}` : 'Offline'}
            </span>
            <span className="net-badge">Testnet</span>
          </div>
        </div>

        {/* Balance */}
        <div className="balance-wrap">
          <div className="balance-big">
            {eddaBal.toLocaleString('en', { maximumFractionDigits: 4 })}
            <span className="balance-ticker">EDDA</span>
          </div>
          <div className="balance-sub">{balance.toLocaleString()} lamports</div>
        </div>

        {/* Address */}
        <div className="addr-row">
          <span className="addr-text">{wallet.publicKey}</span>
          <button className="copy-btn" onClick={copyAddress}>{copied ? '✓' : 'Copy'}</button>
        </div>

        {/* Tabs */}
        <div className="tabs">
          <div className={`tab ${tab === 'send' ? 'active' : ''}`} onClick={() => setTab('send')}>Send</div>
          <div className={`tab ${tab === 'history' ? 'active' : ''}`} onClick={() => setTab('history')}>
            History{txHistory.length > 0 ? ` (${txHistory.length})` : ''}
          </div>
          <div className={`tab ${tab === 'tokens' ? 'active' : ''}`} onClick={() => setTab('tokens')}>
            Tokens{tokens.length > 0 ? ` (${tokens.length})` : ''}
          </div>
        </div>

        {/* Send EDDA */}
        {tab === 'send' && (
          <div className="section">
            <label className="inp-label">Recipient</label>
            <input className="inp inp-mono" placeholder="64-character hex address"
              value={toAddr} onChange={e => { setToAddr(e.target.value); setSendMsg(null); }} />
            <label className="inp-label">
              Amount
              <span style={{ float: 'right', color: 'var(--sub)', cursor: 'pointer', fontWeight: 400 }}
                onClick={() => setAmount(maxSend.toFixed(9))}>
                Max {maxSend.toFixed(4)}
              </span>
            </label>
            <input className="inp" placeholder="0.0000" type="number" min="0" step="0.0001"
              value={amount} onChange={e => { setAmount(e.target.value); setSendMsg(null); }} />
            <div className="fee-note">Network fee: 0.000005 EDDA</div>
            <button className="btn btn-primary" onClick={doSend}
              disabled={sending || !online || !toAddr || !amount}>
              {sending ? 'Sending…' : 'Send EDDA'}
            </button>
            {sendMsg && (
              <div className={`toast toast-${sendMsg.type}`}>
                {sendMsg.text}
                {sendMsg.sig && <div className="mono-sm">{shortSig(sendMsg.sig)}</div>}
              </div>
            )}
            {!online && (
              <div className="toast toast-warn">Node offline — start edda-node on port 8899</div>
            )}
          </div>
        )}

        {/* History */}
        {tab === 'history' && (
          <div className="section">
            {txHistory.length === 0
              ? <div className="empty">No transactions yet</div>
              : txHistory.map(tx => (
                <div className="tx-row" key={tx.signature} style={{ display: 'flex', alignItems: 'center' }}>
                  <div className={`tx-icon ${tx.direction === 'sent' ? 'tx-sent-icon' : 'tx-recv-icon'}`}>
                    {tx.direction === 'sent' ? '↑' : '↓'}
                  </div>
                  <div style={{ flex: 1 }}>
                    <div className="tx-label">{tx.direction === 'sent' ? 'Sent' : 'Received'}</div>
                    <div className="tx-sub">
                      {tx.direction === 'sent' ? shortAddr(tx.to) : shortAddr(tx.from)} · Slot {tx.slot}
                    </div>
                  </div>
                  <div style={{ textAlign: 'right' }}>
                    <div className={tx.direction === 'sent' ? 'tx-amt-neg' : 'tx-amt-pos'}>
                      {tx.direction === 'sent' ? '−' : '+'}{fmtEdda(tx.lamports)}
                    </div>
                    <div className="tx-sub">{new Date(tx.timestamp_ms).toLocaleTimeString()}</div>
                  </div>
                </div>
              ))}
          </div>
        )}

        {/* Tokens */}
        {tab === 'tokens' && (
          <div className="section">
            {tokens.length === 0
              ? <div className="empty">No tokens on this network</div>
              : tokens.map(t => {
                const raw  = tokenBals[t.id] ?? 0;
                const disp = raw / Math.pow(10, t.decimals);
                const open = tokenSendId === t.id;
                return (
                  <div key={t.id}>
                    <div className="tok-row" style={{ cursor: raw > 0 ? 'pointer' : 'default' }}
                      onClick={() => { if (raw > 0) { setTokenSendId(open ? '' : t.id); setTokenSendMsg(null); } }}>
                      <div className="tok-icon">{t.symbol.slice(0, 2)}</div>
                      <div style={{ flex: 1 }}>
                        <div className="tok-name">{t.name}</div>
                        <div className="tok-sym">{t.symbol}</div>
                      </div>
                      <div>
                        <div className="tok-bal">{disp.toLocaleString()}</div>
                        <div className="tok-bal-sub">{raw > 0 ? (open ? 'close ▲' : 'send ▼') : '—'}</div>
                      </div>
                    </div>

                    {open && selTok && (
                      <div className="send-form">
                        <label className="inp-label" style={{ marginTop: 0 }}>Recipient</label>
                        <input className="inp inp-mono" placeholder="64-character hex address"
                          value={tokenToAddr} onChange={e => { setTokenToAddr(e.target.value); setTokenSendMsg(null); }} />
                        <label className="inp-label">
                          Amount
                          <span style={{ float: 'right', color: 'var(--sub)', cursor: 'pointer', fontWeight: 400 }}
                            onClick={() => setTokenAmount((raw / Math.pow(10, t.decimals)).toFixed(t.decimals))}>
                            Max {disp.toLocaleString()}
                          </span>
                        </label>
                        <input className="inp" placeholder="0" type="number" min="0"
                          value={tokenAmount} onChange={e => { setTokenAmount(e.target.value); setTokenSendMsg(null); }} />
                        <button className="btn btn-primary" style={{ marginTop: 12 }}
                          onClick={doSendToken} disabled={tokenSending || !online || !tokenToAddr || !tokenAmount}>
                          {tokenSending ? 'Sending…' : `Send ${t.symbol}`}
                        </button>
                        {tokenSendMsg && (
                          <div className={`toast toast-${tokenSendMsg.type}`}>{tokenSendMsg.text}</div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
          </div>
        )}

        {/* Bottom actions */}
        <div className="section" style={{ borderTop: '1px solid var(--border)', paddingTop: 20 }}>
          <div className="danger-row">
            <button className="btn btn-secondary btn-sm" style={{ flex: 1 }}
              onClick={() => { navigator.clipboard.writeText(wallet.privateKey); alert('Copied! Never share your private key.'); }}>
              Export Key
            </button>
            <button className="btn btn-danger btn-sm" style={{ flex: 1 }} onClick={deleteWallet}>
              Delete
            </button>
          </div>
        </div>
      </div>

      <div className="footer">
        Edda Network · Testnet · eddachain.com<br />
        Keys stored locally only.
      </div>
    </>
  );
}
