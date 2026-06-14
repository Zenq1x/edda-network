'use client';

import { useEffect, useState, useCallback } from 'react';
import {
  getNetworkInfo, getRecentBlocks, getBalance, getTxHistory, listTokens,
  NetworkInfo, BlockInfo, TxInfo, TxHistoryEntry, TokenInfo,
} from '@/lib/rpc';

const LAMPORTS = 1_000_000_000;
const POLL_MS  = 1_500;

function shortH(h: string, n = 8) { return h.slice(0, n) + '…' + h.slice(-n); }
function edda(l: number) { return (l / LAMPORTS).toFixed(4); }
function elapsed(ms: number) {
  const s = Math.floor((Date.now() - ms) / 1000);
  if (s < 2) return 'just now';
  if (s < 60) return `${s}s ago`;
  return `${Math.floor(s / 60)}m ago`;
}
function fmt(n: number) {
  if (n >= 1e6) return (n / 1e6).toFixed(2) + 'M';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + 'K';
  return n.toLocaleString();
}

export default function Explorer() {
  const [info,      setInfo]      = useState<NetworkInfo | null>(null);
  const [blocks,    setBlocks]    = useState<BlockInfo[]>([]);
  const [tokens,    setTokens]    = useState<TokenInfo[]>([]);
  const [selected,  setSelected]  = useState<BlockInfo | null>(null);
  const [search,    setSearch]    = useState('');
  const [balance,   setBalance]   = useState<number | null>(null);
  const [history,   setHistory]   = useState<TxHistoryEntry[] | null>(null);
  const [searchErr, setSearchErr] = useState('');
  const [searching, setSearching] = useState(false);
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
    } catch (e: unknown) {
      setSearchErr(e instanceof Error ? e.message : 'Not found');
    } finally { setSearching(false); }
  }

  const supply  = info ? info.total_supply / LAMPORTS : 0;
  const burned  = info ? info.total_burned / LAMPORTS : 0;
  const height  = blocks[0]?.slot ?? 0;
  const txTotal = blocks.reduce((s, b) => s + b.transaction_count, 0);

  return (
    <>
      <nav className="nav">
        <div className="nav-brand">
          Edda <em>Explorer</em>
        </div>
        <div className="nav-status">
          {info ? (
            <><span className="pulse-dot" />{info.validators} validator{info.validators !== 1 ? 's' : ''}</>
          ) : (
            'connecting...'
          )}
        </div>
      </nav>

      <main className="shell page">

        {/* Stats */}
        <div className="stats">
          {[
            { label: 'Block Height',  val: fmt(height),  sub: 'slots produced',    cls: 'c-gold'  },
            { label: 'Transactions',  val: fmt(txTotal), sub: 'in last 50 blocks', cls: 'c-blue'  },
            { label: 'Total Supply',  val: fmt(supply),  sub: 'EDDA minted',       cls: 'c-text'  },
            { label: 'Fees Burned',   val: fmt(burned),  sub: 'EDDA destroyed',    cls: 'c-green' },
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
              <span>Live Blocks</span>
              <span className="live-badge"><span className="pulse-dot" />live</span>
            </div>
            <table className="block-table">
              <thead>
                <tr>
                  <th>Slot</th>
                  <th>Hash</th>
                  <th>Txs</th>
                  <th>Burned</th>
                  <th>Age</th>
                </tr>
              </thead>
              <tbody>
                {blocks.length === 0 && (
                  <tr>
                    <td colSpan={5} style={{ textAlign: 'center', color: 'var(--sub)', padding: '40px 0', fontFamily: 'var(--fm)', fontSize: 11 }}>
                      Waiting for blocks...
                    </td>
                  </tr>
                )}
                {blocks.map(b => (
                  <tr
                    key={b.slot}
                    onClick={() => setSelected(selected?.slot === b.slot ? null : b)}
                    className={selected?.slot === b.slot ? 'selected-row' : ''}
                  >
                    <td><span className="slot-num">{b.slot}</span></td>
                    <td><span className="mono" style={{ color: 'var(--sub)' }}>{shortH(b.blockhash)}</span></td>
                    <td>
                      {b.transaction_count > 0
                        ? <span className="tx-count">{b.transaction_count}</span>
                        : <span style={{ color: 'var(--dim)' }}>—</span>}
                    </td>
                    <td><span className="mono" style={{ color: 'var(--sub)' }}>{b.fees_burned > 0 ? b.fees_burned.toLocaleString() : '—'}</span></td>
                    <td><span className="mono" style={{ color: 'var(--sub)' }}>{elapsed(b.timestamp_ms)}</span></td>
                  </tr>
                ))}
              </tbody>
            </table>

            {/* Block detail */}
            {selected && (
              <div className="block-detail">
                <div className="detail-header">
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 16 }}>
                    <span style={{ fontWeight: 500, fontSize: 14 }}>Block {selected.slot}</span>
                    <span style={{ color: 'var(--sub)', fontFamily: 'var(--fm)', fontSize: 10 }}>
                      {new Date(selected.timestamp_ms).toLocaleTimeString()}
                    </span>
                  </div>
                  <button className="close-btn" onClick={() => setSelected(null)}>✕</button>
                </div>
                <div className="detail-meta">
                  {[
                    ['Hash',   selected.blockhash],
                    ['Parent', selected.parent_blockhash],
                    ['Leader', selected.leader],
                  ].map(([k, v]) => (
                    <div className="meta-row" key={k}>
                      <span className="meta-key">{k}</span>
                      <span className="meta-val">{shortH(v, 14)}</span>
                    </div>
                  ))}
                </div>
                {selected.transactions.length === 0 ? (
                  <div style={{ padding: '16px 0', color: 'var(--sub)', fontFamily: 'var(--fm)', fontSize: 11 }}>
                    No transactions
                  </div>
                ) : (
                  <div>
                    <div className="section-label">
                      {selected.transactions.length} transaction{selected.transactions.length !== 1 ? 's' : ''}
                    </div>
                    {selected.transactions.map((tx: TxInfo) => (
                      <div className="tx-detail-row" key={tx.signature}>
                        <div className="tx-sig">{shortH(tx.signature, 12)}</div>
                        {tx.transfer ? (
                          <div className="tx-transfer">
                            <span className="addr-chip">{shortH(tx.transfer.from, 6)}</span>
                            <span className="arrow">→</span>
                            <span className="addr-chip">{shortH(tx.transfer.to, 6)}</span>
                            <span className="amount-chip">{edda(tx.transfer.lamports)} EDDA</span>
                            <span className="fee-chip">fee {tx.fee.toLocaleString()}</span>
                          </div>
                        ) : (
                          <div style={{ color: 'var(--sub)', fontSize: 11, fontFamily: 'var(--fm)' }}>
                            Smart contract call
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Right column */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 40 }}>

            {/* Account lookup */}
            <div className="panel">
              <div className="panel-h">Account Lookup</div>
              <div className="search-box">
                <input
                  className="search-input"
                  placeholder="Public key (64 hex chars)"
                  value={search}
                  onChange={e => setSearch(e.target.value)}
                  onKeyDown={e => e.key === 'Enter' && doSearch()}
                />
                <button className="search-btn" onClick={doSearch} disabled={searching}>
                  {searching ? 'Searching...' : 'Look up'}
                </button>
                {searchErr && <div className="error-msg">{searchErr}</div>}
              </div>
              {balance !== null && (
                <div className="result-box">
                  <div className="result-label">Balance</div>
                  <div className="result-val">{edda(balance)} <span style={{ fontSize: 14, color: 'var(--sub)', fontWeight: 300 }}>EDDA</span></div>
                  <div style={{ color: 'var(--sub)', fontFamily: 'var(--fm)', fontSize: 10, marginTop: 6 }}>
                    {balance.toLocaleString()} lamports
                  </div>
                  <div className="result-addr">{search}</div>
                </div>
              )}
              {history !== null && (
                <div>
                  <div className="section-label" style={{ padding: '12px 0 0' }}>
                    History ({history.length})
                  </div>
                  {history.length === 0 ? (
                    <div style={{ padding: '12px 0', color: 'var(--sub)', fontFamily: 'var(--fm)', fontSize: 11 }}>
                      No transactions found
                    </div>
                  ) : history.map(tx => (
                    <div className="hist-row" key={tx.signature}>
                      <div>
                        <div className="hist-dir" style={{ color: tx.direction === 'sent' ? 'var(--red)' : 'var(--green)' }}>
                          {tx.direction === 'sent' ? '↑ Sent' : '↓ Received'}
                        </div>
                        <div className="hist-addr">
                          {tx.direction === 'sent' ? `→ ${shortH(tx.to, 6)}` : `← ${shortH(tx.from, 6)}`}
                        </div>
                        <div className="hist-slot">slot {tx.slot}</div>
                      </div>
                      <div>
                        <div className="hist-amt" style={{ color: tx.direction === 'sent' ? 'var(--red)' : 'var(--green)' }}>
                          {tx.direction === 'sent' ? '−' : '+'}{edda(tx.lamports)}
                        </div>
                        <div className="hist-time">{new Date(tx.timestamp_ms).toLocaleTimeString()}</div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Token Registry */}
            <div className="panel">
              <div className="panel-h">
                <span>Tokens</span>
                <span>{tokens.length}</span>
              </div>
              {tokens.length === 0 ? (
                <div style={{ padding: '16px 0', color: 'var(--sub)', fontFamily: 'var(--fm)', fontSize: 11 }}>
                  No tokens yet
                </div>
              ) : tokens.map(t => (
                <div className="tok-row" key={t.id}>
                  <div>
                    <div className="tok-name">{t.name}</div>
                    <div className="tok-sym">{t.symbol}</div>
                    <div className="tok-id">{shortH(t.id, 8)}</div>
                  </div>
                  <div>
                    <div className="tok-supply">{fmt(t.total_supply / Math.pow(10, t.decimals))}</div>
                    <div className="tok-max">/ {t.max_supply > 0 ? fmt(t.max_supply / Math.pow(10, t.decimals)) : '∞'}</div>
                  </div>
                </div>
              ))}
            </div>

            {/* Network info */}
            <div className="panel">
              <div className="panel-h">Network</div>
              {[
                ['Consensus',    'PoH + Tower BFT'],
                ['Contracts',    'WebAssembly'],
                ['Max Supply',   '500M EDDA'],
                ['Slot time',    '400 ms'],
                ['Fees',         '50% burned'],
                ['RPC',          'rpc.eddachain.com'],
              ].map(([k, v]) => (
                <div className="net-row" key={k}>
                  <span className="net-key">{k}</span>
                  <span className="net-val">{v}</span>
                </div>
              ))}
            </div>

          </div>
        </div>

        <div className="footer">
          Edda Network · EDDA · eddachain.com
        </div>
      </main>
    </>
  );
}
