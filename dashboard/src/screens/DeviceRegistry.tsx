import React from 'react';
import { Smartphone, Shield, MapPin, MoreVertical } from 'lucide-react';
import { motion } from 'framer-motion';

const DeviceRegistry: React.FC = () => {
    const devices = [
        { id: '1', did: 'did:malama:sensor:8b3f4...', type: 'CO2 Monitor', status: 'ACTIVE', location: 'Nairobi, Kenya', reputation: 98 },
        { id: '2', did: 'did:malama:sensor:f92a1...', type: 'Soil Sensor', status: 'ACTIVE', location: 'Bali, Indonesia', reputation: 95 },
        { id: '3', did: 'did:malama:sensor:a44d8...', type: 'Air Quality', status: 'OFFLINE', location: 'Amazonas, Brazil', reputation: 82 },
    ];

    return (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '20px' }}>
            {devices.map((device, index) => (
                <motion.div
                    key={device.id}
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: index * 0.05 }}
                    className="glass"
                    style={{ padding: '24px', borderRadius: '20px', display: 'flex', alignItems: 'center', gap: '32px' }}
                >
                    <div style={{ width: '56px', height: '56px', borderRadius: '16px', background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                        <Smartphone size={24} color={device.status === 'ACTIVE' ? 'var(--brand-primary)' : 'var(--text-tertiary)'} />
                    </div>

                    <div style={{ flex: 1 }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '8px' }}>
                            <span className="mono" style={{ fontWeight: 700, fontSize: '0.9rem', letterSpacing: '-0.02em' }}>{device.did}</span>
                            <span style={{
                                fontSize: '0.65rem',
                                padding: '4px 10px',
                                borderRadius: '12px',
                                background: device.status === 'ACTIVE' ? 'rgba(0, 255, 156, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                color: device.status === 'ACTIVE' ? 'var(--brand-primary)' : 'var(--text-tertiary)',
                                fontWeight: 800,
                                border: device.status === 'ACTIVE' ? '1px solid rgba(0, 255, 156, 0.2)' : '1px solid transparent'
                            }}>
                                {device.status}
                            </span>
                        </div>
                        <div style={{ display: 'flex', gap: '24px', color: 'var(--text-secondary)', fontSize: '0.75rem', fontWeight: 500 }}>
                            <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}><Shield size={14} color="var(--brand-tertiary)" /> {device.type}</span>
                            <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}><MapPin size={14} color="var(--brand-secondary)" /> {device.location}</span>
                            <span style={{ color: 'var(--text-tertiary)' }}>Reputation: <span style={{ color: device.reputation > 90 ? 'var(--brand-primary)' : 'var(--text-primary)' }}>{device.reputation}%</span></span>
                        </div>
                    </div>

                    <div style={{ textAlign: 'right' }}>
                        <div style={{ color: 'var(--text-tertiary)', fontSize: '0.65rem', fontWeight: 800, marginBottom: '6px', letterSpacing: '0.05em' }}>LAST_UPLINK</div>
                        <div className="mono" style={{ fontSize: '0.875rem', fontWeight: 600 }}>14:44:03</div>
                    </div>

                    <button style={{
                        padding: '10px 20px',
                        borderRadius: '10px',
                        border: '1px solid var(--border-color)',
                        fontSize: '0.75rem',
                        fontWeight: 700,
                        background: 'rgba(255,255,255,0.03)',
                        transition: 'all 0.2s'
                    }}>
                        MANAGE_CERT
                    </button>

                    <MoreVertical size={20} color="var(--text-tertiary)" style={{ cursor: 'pointer' }} />
                </motion.div>
            ))}
        </div>
    );
};

export default DeviceRegistry;
