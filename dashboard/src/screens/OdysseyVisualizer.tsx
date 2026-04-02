import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
    Box,
    Share2,
    Anchor,
    CheckCircle2,
    ArrowRight,
    Database,
    Zap,
    Cpu,
    Activity,
    Shield
} from 'lucide-react';

const odysseyStages = [
    { id: 'birth', icon: Zap, label: 'Data Birth', description: 'Sensor signed reading', color: 'var(--brand-primary)', meta: 'ED25519_SIG_OK' },
    { id: 'batch', icon: Box, label: 'Batching', description: 'Merkle Tree construction', color: 'var(--brand-secondary)', meta: 'ROOT: 0x8f2c...4a' },
    { id: 'consensus', icon: Share2, label: 'Consensus', description: '2-of-3 Validator approval', color: 'var(--brand-primary)', meta: 'QUORUM_REACHED' },
    { id: 'anchor', icon: Anchor, label: 'Anchoring', description: 'Multi-chain recording', color: 'var(--brand-secondary)', meta: 'TX_CONFIRMED' },
    { id: 'registry', icon: Database, label: 'Registry', description: 'International reporting', color: 'var(--brand-tertiary)', meta: 'UN_SDG_COMPLIANT' },
    { id: 'settlement', icon: CheckCircle2, label: 'Settlement', description: 'Carbon token minting', color: 'var(--brand-primary)', meta: 'LCO2_MINTED' },
];

const OdysseyVisualizer: React.FC = () => {
    const [activeLogs, setActiveLogs] = useState<string[]>([]);
    const [hoveredStage, setHoveredStage] = useState<string | null>(null);

    useEffect(() => {
        const interval = setInterval(() => {
            const logs = [
                `[${new Date().toLocaleTimeString()}] PING VALIDATOR_NODE_03... OK`,
                `[${new Date().toLocaleTimeString()}] BATCH_SIZE: 128 READINGS`,
                `[${new Date().toLocaleTimeString()}] CALCULATING_LSH_FINGERPRINT: 0x${Math.random().toString(16).slice(2, 10)}`,
                `[${new Date().toLocaleTimeString()}] CARDANO_TX_ID: ${Math.random().toString(36).slice(2, 20)}`,
                `[${new Date().toLocaleTimeString()}] CROSS_CHAIN_SYNC: BASE/HEDERA/CELO... SYNCED`,
            ];
            setActiveLogs(prev => [logs[Math.floor(Math.random() * logs.length)], ...prev].slice(0, 5));
        }, 3000);
        return () => clearInterval(interval);
    }, []);

    return (
        <div style={{ padding: '24px', position: 'relative' }}>
            <div className="glass cyber-grid" style={{ padding: '64px 48px', borderRadius: '40px', overflow: 'hidden', position: 'relative' }}>

                {/* Header */}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '64px', position: 'relative', zIndex: 2 }}>
                    <div>
                        <h3 className="mono" style={{ fontSize: '0.875rem', color: 'var(--brand-primary)', marginBottom: '8px', letterSpacing: '0.2em' }}>
                            MALAMA_PIPELINE_EXPLORER
                        </h3>
                        <h2 style={{ fontSize: '2rem', fontWeight: 900, letterSpacing: '-0.02em' }}>Data Odyssey</h2>
                    </div>
                    <div style={{ textAlign: 'right' }}>
                        <div style={{ display: 'flex', gap: '24px' }}>
                            <HeaderStat icon={Cpu} label="NODES" value="12" />
                            <HeaderStat icon={Activity} label="UPTIME" value="99.99%" />
                            <HeaderStat icon={Shield} label="BFT_MODE" value="LITE" />
                        </div>
                    </div>
                </div>

                {/* Main Pipeline Area */}
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', position: 'relative', minHeight: '300px' }}>

                    {/* Background Connector & Pulses */}
                    <div style={{ position: 'absolute', top: '40px', left: '60px', right: '60px', height: '2px', background: 'rgba(255,255,255,0.05)', zIndex: 0 }}>
                        <div className="data-pulse" style={{ animationDelay: '0s' }} />
                        <div className="data-pulse" style={{ animationDelay: '1.5s' }} />
                        <div className="data-pulse" style={{ animationDelay: '3s' }} />
                    </div>

                    {odysseyStages.map((stage, index) => (
                        <motion.div
                            key={stage.id}
                            onMouseEnter={() => setHoveredStage(stage.id)}
                            onMouseLeave={() => setHoveredStage(null)}
                            initial={{ opacity: 0, scale: 0.8 }}
                            animate={{
                                opacity: 1,
                                scale: hoveredStage === stage.id ? 1.05 : 1,
                                y: hoveredStage === stage.id ? -5 : 0
                            }}
                            transition={{ type: 'spring', stiffness: 300, damping: 20 }}
                            style={{
                                display: 'flex',
                                flexDirection: 'column',
                                alignItems: 'center',
                                width: '130px',
                                zIndex: 2,
                                cursor: 'pointer'
                            }}
                        >
                            <div
                                style={{
                                    width: '80px',
                                    height: '80px',
                                    borderRadius: '24px',
                                    display: 'flex',
                                    alignItems: 'center',
                                    justifyContent: 'center',
                                    background: hoveredStage === stage.id ? 'rgba(0, 255, 156, 0.1)' : 'rgba(0,0,0,0.4)',
                                    border: `1px solid ${hoveredStage === stage.id ? 'var(--brand-primary)' : `${stage.color}33`}`,
                                    marginBottom: '20px',
                                    boxShadow: hoveredStage === stage.id ? `0 0 30px ${stage.color}33` : 'none',
                                    transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)'
                                }}
                            >
                                <stage.icon size={32} color={hoveredStage === stage.id ? 'var(--brand-primary)' : stage.color} />
                            </div>

                            <div style={{ fontWeight: 800, fontSize: '0.9rem', marginBottom: '6px', textAlign: 'center' }}>
                                {stage.label}
                            </div>

                            <AnimatePresence mode="wait">
                                {hoveredStage === stage.id ? (
                                    <motion.div
                                        initial={{ opacity: 0, y: 5 }}
                                        animate={{ opacity: 1, y: 0 }}
                                        className="mono"
                                        style={{ fontSize: '0.6rem', color: 'var(--brand-primary)', fontWeight: 700 }}
                                    >
                                        {stage.meta}
                                    </motion.div>
                                ) : (
                                    <motion.div
                                        initial={{ opacity: 0 }}
                                        animate={{ opacity: 1 }}
                                        style={{ fontSize: '0.7rem', color: 'var(--text-tertiary)', fontWeight: 500, textAlign: 'center' }}
                                    >
                                        {stage.description}
                                    </motion.div>
                                )}
                            </AnimatePresence>
                        </motion.div>
                    ))}
                </div>

                {/* Live Console & Log Area */}
                <div style={{ marginTop: '64px', display: 'grid', gridTemplateColumns: '1.5fr 1fr', gap: '32px', position: 'relative', zIndex: 2 }}>

                    {/* Live Logs */}
                    <div className="glass" style={{ padding: '24px', borderRadius: '20px', background: 'rgba(0,0,0,0.4)', minHeight: '160px' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '16px' }}>
                            <span className="mono" style={{ fontSize: '0.65rem', fontWeight: 800, color: 'var(--text-tertiary)' }}>ODYSSEY_RUNTIME_LOGS</span>
                            <span className="mono" style={{ fontSize: '0.65rem', color: 'var(--brand-primary)' }}>STATUS: STREAMING</span>
                        </div>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                            {activeLogs.map((log, i) => (
                                <motion.div
                                    key={i}
                                    initial={{ opacity: 0, x: -10 }}
                                    animate={{ opacity: 1 - i * 0.15, x: 0 }}
                                    className="mono"
                                    style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}
                                >
                                    {log}
                                </motion.div>
                            ))}
                        </div>
                    </div>

                    {/* Batch Progress */}
                    <div className="glass" style={{ padding: '24px', borderRadius: '20px', background: 'rgba(0,255,156,0.02)', border: '1px solid rgba(0,255,156,0.1)' }}>
                        <div style={{ fontSize: '0.75rem', fontWeight: 800, marginBottom: '20px', letterSpacing: '0.05em' }}>CURRENT_BATCH_INTEGRITY</div>
                        <div style={{ marginBottom: '12px', display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
                            <span style={{ fontSize: '2rem', fontWeight: 900 }}>99.8%</span>
                            <span className="mono" style={{ fontSize: '0.7rem', color: 'var(--brand-primary)', marginBottom: '6px' }}>SECURE</span>
                        </div>
                        <div style={{ height: '6px', background: 'rgba(255,255,255,0.05)', borderRadius: '3px', position: 'relative', overflow: 'hidden' }}>
                            <motion.div
                                style={{ height: '100%', background: 'var(--brand-primary)', width: '99.8%' }}
                                animate={{ opacity: [0.6, 1, 0.6] }}
                                transition={{ duration: 2, repeat: Infinity }}
                            />
                        </div>
                    </div>

                </div>

            </div>
        </div>
    );
};

const HeaderStat = ({ icon: Icon, label, value }: any) => (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-tertiary)', fontSize: '0.65rem', fontWeight: 800, marginBottom: '4px' }}>
            <Icon size={12} /> {label}
        </div>
        <div className="mono" style={{ fontSize: '1rem', fontWeight: 700 }}>{value}</div>
    </div>
);

export default OdysseyVisualizer;
