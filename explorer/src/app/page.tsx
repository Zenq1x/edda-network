'use client';

import { useEffect, useState, useCallback } from 'react';
import {
  getNetworkInfo, getRecentBlocks, getBalance, getTxHistory, listTokens,
  NetworkInfo, BlockInfo, TxInfo, TxHistoryEntry, TokenInfo,
} from '@/lib/rpc';

const LAMPORTS   = 1_000_000_000;
const POLL_MS    = 1_500;

function shortH(h: string, n = 8) { return h.slice(0, n) + '…' + h.slice(-n); }
function edda(l: number)  { return (l / LAMPORTS).toFixed(4); }
function elapsed(ms: number) {
  const s = Math.floor((Date.now() - ms) / 1000);
  if (s < 2) return 'just now'; if (s < 60) return `${s}s ago`;
  return `${Math.floor(s / 60)}m ago`;
}
function fmt(n: number) {
  if (n >= 1e6) return (n / 1e6).toFixed(2) + 'M';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + 'K';
  return n.toLocaleString();
}

export default function Explorer() {
  const [info,       setInfo]       = useState<NetworkInfo | null>(null);
  const [blocks,     setBlocks]     = useState<BlockInfo[]>([]);
  const [tokens,     setTokens]     = useState<TokenInfo[]>([]);
  const [selected,   setSelected]   = useState<BlockInfo | null>(null);
  const [search,     setSearch]     = useState('');
  const [balance,    setBalance]    = useState<number | null>(null);
  const [history,    setHistory]    = useState<TxHistoryEntry[] | null>(null);
  const [searchErr,  setSearchErr]  = useState('');
  const [searching,  setSearching]  = useState(false);
  const [, setTick] = useState(0);

  const poll = useCallback(async () => {
    try {
      const [ni, recent, toks] = await Promise.all([
        getNetworkInfo(), getRecentBlocks(50), listTokens(),
      ]);
      setInfo(ni);
      setBlocks(recent);
      setTokens(toks);
    } catch { /* node offline */ }
  }, []);

  useEffect(() => {
    poll();
    const id  = setInterval(poll, POLL_MS);
    const tid = setInterval(() => setTick(t => t + 1), 5000);
    return () => { clearInterval(id); clearInterval(tid); };
  }, [poll]);

  async function doSearch() {
    const key = search.trim();
    if (!key) return;
    setSearching(true); setSearchErr(''); setBalance(null); setHistory(null);
    try {
      const [bal, hist] = await Promise.all([getBalance(key), getTxHistory(key, 20)]);
      setBalance(bal);
      setHistory(hist);
    } catch (e: unknown) { setSearchErr(e instanceof Error ? e.message : 'Not found'); }
    finally { setSearching(false); }
  }

  const supply  = info ? info.total_supply / LAMPORTS : 0;
  const burned  = info ? info.total_burned / LAMPORTS : 0;
  const height  = blocks[0]?.slot ?? 0;
  const txTotal = blocks.reduce((s, b) => s + b.transaction_count, 0);

  return (
    <>
      <nav className="nav">
        <div className="nav-logo">
          <span className="nav-dot" />
          <div>
            <div>Edda Explorer</div>
            <div className="nav-sub">eddachain.com · Testnet</div>
          </div>
        </div>
        <div style={{ color: 'var(--muted)', fontSize: 12 }}>
          {info ? `${info.validators} validators · live` : 'connecting...'}
        </div>
      </nav>

      <main className="shell page">

        {/* Stats */}
        <div className="stats">
          {[
            { label: 'Block Height',  val: fmt(height),    sub: 'slots produced',    cls: 'accent'  },
            { label: 'Transactions',  val: fmt(txTotal),   sub: 'in last 50 blocks', cls: 'accent2' },
            { label: 'Total Supply',  val: fmt(supply),    sub: 'EDDA minted',       cls: ''        },
            { label: 'Fees Burned',   val: fmt(burned),    sub: 'EDDA destroyed',    cls: 'green'   },
          ].map(({ label, val, sub, cls }) => (
            <div className="stat-card" key={label}>
              <div className="stat-label">{label}</div>
              <div className={`stat-val ${cls}`}>{val}</div>
              <div className="stat-sub">{sub}</div>
            </div>
          ))}
        </div>

        <div className="panels">

          {/* Block feed */}
          <div className="panel">
            <div className="panel-h">
              <span><span className="live-dot" />Live Blocks</span>
              <span style={{ color: 'var(--muted)', fontSize: 12 }}>click a block to inspect</span>
            </div>
            <table className="block-table">
              <thead>
                <tr>
                  <th>Slot</th><th>Blockhash</th><th>Txs</th><th>Burned</th><th>Age</th>
                </tr>
              </thead>
              <tbody>
                {blocks.length === 0 && (
                  <tr><td colSpan={5} style={{ textAlign: 'center', color: 'var(--muted)', padding: 32 }}>
                    Waiting for blocks...
                  </td></tr>
                )}
                {blocks.map(b => (
                  <tr
                    key={b.slot}
                    onClick={() => setSelected(selected?.slot === b.slot ? null : b)}
                    style={{ cursor: 'pointer' }}
                    className={selected?.slot === b.slot ? 'selected-row' : ''}
                  >
                    <td><span className="slot-badge">#{b.slot}</span></td>
                    <td className="mono accent">{shortH(b.blockhash)}</td>
                    <td>
                      {b.transaction_count > 0
                        ? <span className="tx-badge">{b.transaction_count}</span>
                        : <span style={{ color: 'var(--muted)' }}>—</span>}
                    </td>
                    <td style={{ color: 'var(--muted)', fontSize: 12 }}>
                      {b.fees_burned > 0 ? b.fees_burned.toLocaleString() : '—'}
                    </td>
                    <td style={{ color: 'var(--muted)' }}>{elapsed(b.timestamp_ms)}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            {/* Block detail */}
            {selected && (
              <div className="block-detail">
                <div className="detail-header">
                  <div>
                    <span className="slot-badge" style={{ fontSize: 14 }}>Block #{selected.slot}</span>
                    <span style={{ marginLeft: 12, color: 'var(--muted)', fontSize: 12 }}>
                      {new Date(selected.timestamp_ms).toLocaleTimeString()}
                    </span>
                  </div>
                  <button className="close-btn" onClick={() => setSelected(null)}>✕</button>
                </div>
                <div className="detail-meta">
                  {[['Hash', selected.blockhash], ['Parent', selected.parent_blockhash], ['Leader', selected.leader]].map(([k, v]) => (
                    <div className="meta-row" key={k}>
                      <span className="meta-key">{k}</span>
                      <span className="meta-val mono">{shortH(v, 12)}</span>
                    </div>
                  ))}
                </div>
                {selected.transactions.length === 0 ? (
                  <div style={{ padding: '20px 24px', color: 'var(--muted)', fontSize: 13 }}>
                    No transactions in this block
                  </div>
                ) : (
                  <div>
                    <div style={{ padding: '12px 24px 4px', color: 'var(--muted)', fontSize: 11,
                                  textTransform: 'uppercase', letterSpacing: '.6px' }}>
                      {selected.transactions.length} Transaction{selected.transactions.length !== 1 ? 's' : ''}
                    </div>
                    {selected.transactions.map((tx: TxInfo) => (
                      <div className="tx-detail-row" key={tx.signature}>
                        <div className="tx-sig mono">{shortH(tx.signature, 10)}</div>
                        {tx.transfer ? (
                          <div className="tx-transfer">
                            <span className="addr-chip">{shortH(tx.transfer.from, 6)}</span>
                            <span className="arrow">→</span>
                            <span className="addr-chip">{shortH(tx.transfer.to, 6)}</span>
                            <span className="amount-chip">{edda(tx.transfer.lamports)} EDDA</span>
                            <span className="fee-chip">fee {tx.fee.toLocaleString()} lp</span>
                          </div>
                        ) : (
                          <div style={{ color: 'var(--muted)', fontSize: 12 }}>Smart contract call</div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Right column */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>

            {/* Account lookup */}
            <div className="panel">
              <div className="panel-h">Account Lookup</div>
              <div className="search-box">
                <input
                  className="search-input"
                  placeholder="Paste public key (hex)..."
                  value={search}
                  onChange={e => setSearch(e.target.value)}
                  onKeyDown={e => e.key === 'Enter' && doSearch()}
                />
                <button className="search-btn" onClick={doSearch} disabled={searching}>
                  {searching ? 'Searching...' : 'Look up account'}
                </button>
                {searchErr && <div className="error-msg">{searchErr}</div>}
              </div>
              {balance !== null && (
                <div className="result-box">
                  <div className="result-label">Balance</div>
                  <div className="result-val accent">{edda(balance)} EDDA</div>
                  <div style={{ color: 'var(--muted)', fontSize: 12, marginTop: 4 }}>
                    {balance.toLocaleString()} lamports
                  </div>
                  <div className="result-addr">{search}</div>
                </div>
              )}
              {/* Tx History */}
              {history !== null && (
                <div style={{ borderTop: '1px solid var(--border)' }}>
                  <div style={{ padding: '10px 20px 4px', color: 'var(--muted)', fontSize: 11,
                                textTransform: 'uppercase', letterSpacing: '.6px' }}>
                    Transaction History ({history.length})
                  </div>
                  {history.length === 0 ? (
                    <div style={{ padding: '12px 20px', color: 'var(--muted)', fontSize: 12 }}>No transactions found</div>
                  ) : history.map((tx) => (
                    <div key={tx.signature} style={{
                      padding: '10px 20px', borderBottom: '1px solid var(--border)',
                      display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                    }}>
                      <div>
                        <div style={{ fontSize: 11, color: tx.direction === 'sent' ? 'var(--red)' : 'var(--green)',
                                      fontWeight: 700, marginBottom: 2 }}>
                          {tx.direction === 'sent' ? '↑ Sent' : '↓ Received'}
                        </div>
                        <div className="mono" style={{ fontSize: 10, color: 'var(--muted)' }}>
                          {tx.direction === 'sent'
                            ? `→ ${shortH(tx.to, 6)}`
                            : `← ${shortH(tx.from, 6)}`}
                        </div>
                        <div className="mono" style={{ fontSize: 10, color: 'var(--muted)' }}>
                          slot #{tx.slot}
                        </div>
                      </div>
                      <div style={{ textAlign: 'right' }}>
                        <div style={{
                          fontWeight: 700, fontSize: 13,
                          color: tx.direction === 'sent' ? 'var(--red)' : 'var(--green)',
                        }}>
                          {tx.direction === 'sent' ? '−' : '+'}{edda(tx.lamports)} EDDA
                        </div>
                        <div style={{ color: 'var(--muted)', fontSize: 10 }}>
                          {new Date(tx.timestamp_ms).toLocaleTimeString()}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Token Registry */}
            <div className="panel">
              <div className="panel-h">
                <span>Token Registry</span>
                <span style={{ color: 'var(--muted)', fontSize: 12 }}>{tokens.length} token{tokens.length !== 1 ? 's' : ''}</span>
              </div>
              {tokens.length === 0 ? (
                <div style={{ padding: '20px', color: 'var(--muted)', fontSize: 12 }}>
                  No tokens created yet
                </div>
              ) : tokens.map(t => (
                <div key={t.id} style={{
                  padding: '12px 20px', borderBottom: '1px solid var(--border)',
                  display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                }}>
                  <div>
                    <div style={{ fontWeight: 700, fontSize: 13 }}>
                      {t.name} <span style={{ color: 'var(--accent)', fontSize: 11 }}>{t.symbol}</span>
                    </div>
                    <div className="mono" style={{ fontSize: 10, color: 'var(--muted)', marginTop: 2 }}>
                      {shortH(t.id, 8)}
                    </div>
                  </div>
                  <div style={{ textAlign: 'right', fontSize: 12 }}>
                    <div style={{ color: 'var(--text)', fontWeight: 600 }}>
                      {fmt(t.total_supply / Math.pow(10, t.decimals))}
                    </div>
                    <div style={{ color: 'var(--muted)' }}>
                      / {t.max_supply > 0 ? fmt(t.max_supply / Math.pow(10, t.decimals)) : '∞'} max
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {/* Network info */}
            <div className="panel">
              <div className="panel-h">Network</div>
              <div style={{ padding: '4px 0' }}>
                {[
                  ['Consensus',       'PoH + Tower BFT'],
                  ['Smart Contracts', 'WebAssembly (WASM)'],
                  ['Max Supply',      '500,000,000 EDDA'],
                  ['Slot time',       '400 ms'],
                  ['Fee model',       'Base burned + tip'],
                  ['RPC',             '127.0.0.1:8899'],
                ].map(([k, v]) => (
                  <div key={k} style={{ display: 'flex', justifyContent: 'space-between',
                    padding: '10px 20px', borderBottom: '1px solid var(--border)', fontSize: 12 }}>
                    <span style={{ color: 'var(--muted)' }}>{k}</span>
                    <span style={{ fontWeight: 500 }}>{v}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className="footer">
          Edda Network · Ticker: EDDA · eddachain.com · Built in Rust &amp; TypeScript
        </div>
      </main>
    </>
  );
}
