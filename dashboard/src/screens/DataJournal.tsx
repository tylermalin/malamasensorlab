import React from 'react';
import { CheckCircle2, Share2, Database, Zap } from 'lucide-react';

const DataJournal: React.FC = () => {
    const events = [
        { id: 1, type: 'SETTLEMENT', batch: 'B721-X', chain: 'Hedera', status: 'CONFIRMED', time: '12:45:01' },
        { id: 2, type: 'CONSENSUS', batch: 'B721-X', chain: 'Internal', status: 'REACHED', time: '12:44:58' },
        { id: 3, type: 'INGESTION', sensor: 'S-721', value: '22.5', status: 'VERIFIED', time: '12:44:50' },
        { id: 4, type: 'ANCHOR', cid: 'QmX...2b', chain: 'Cardano', status: 'TESTNET', time: '12:40:12' },
    ];

    return (
        <div style={{ position: 'relative' }}>
            <div style={{ position: 'absolute', left: '20px', top: 0, bottom: 0, width: '2px', background: 'var(--border-color)', zIndex: 0 }} />

            <div style={{ display: 'flex', flexDirection: 'column', gap: '32px' }}>
                {events.map((event) => (
                    <div key={event.id} style={{ display: 'flex', gap: '24px', position: 'relative', zIndex: 1 }}>
                        <div style={{
                            width: '42px', height: '42px', borderRadius: '50%', background: 'var(--bg-secondary)',
                            border: '2px solid var(--border-color)', display: 'flex', alignItems: 'center', justifyContent: 'center'
                        }}>
                            {event.type === 'SETTLEMENT' && <Zap size={18} color="var(--brand-secondary)" />}
                            {event.type === 'CONSENSUS' && <Database size={18} color="var(--brand-primary)" />}
                            {event.type === 'INGESTION' && <CheckCircle2 size={18} color="var(--brand-primary)" />}
                            {event.type === 'ANCHOR' && <Share2 size={18} color="var(--brand-tertiary)" />}
                        </div>

                        <div className="glass" style={{ flex: 1, padding: '20px', borderRadius: '16px' }}>
                            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
                                <div style={{ fontWeight: 700, fontSize: '0.875rem' }}>{event.type} EVENT</div>
                                <div className="mono" style={{ fontSize: '0.75rem', color: 'var(--text-tertiary)' }}>{event.time}</div>
                            </div>

                            <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
                                {event.batch && <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>BATCH: <span className="mono">{event.batch}</span></div>}
                                {event.chain && <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>CHAIN: {event.chain}</div>}
                                {event.sensor && <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>SENSOR: {event.sensor}</div>}
                                <div style={{ marginLeft: 'auto', fontSize: '0.65rem', fontWeight: 800, color: 'var(--brand-primary)' }}>{event.status}</div>
                            </div>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
};

export default DataJournal;
