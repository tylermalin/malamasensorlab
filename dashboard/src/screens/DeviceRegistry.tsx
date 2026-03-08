import React from 'react';
import { Smartphone, Shield, MapPin } from 'lucide-react';

const DeviceRegistry: React.FC = () => {
    const devices = [
        { id: '1', did: 'did:cardano:sensor:7d2a9f...e4b1c', type: 'Air Quality', status: 'ACTIVE', location: 'Honolulu, HI' },
        { id: '2', did: 'did:cardano:sensor:a82b1c...9f2d1', type: 'Soil Moisture', status: 'OFFLINE', location: 'Kahului, HI' },
        { id: '3', did: 'did:cardano:sensor:3e1d4f...c5a2e', type: 'Humidity', status: 'ACTIVE', location: 'Hilo, HI' },
    ];

    return (
        <div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '16px' }}>
                {devices.map((device) => (
                    <div key={device.id} className="glass" style={{ padding: '24px', borderRadius: '16px', display: 'flex', alignItems: 'center', gap: '24px' }}>
                        <div style={{ width: '48px', height: '48px', borderRadius: '12px', background: 'var(--bg-tertiary)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                            <Smartphone size={24} color={device.status === 'ACTIVE' ? 'var(--brand-primary)' : 'var(--text-tertiary)'} />
                        </div>

                        <div style={{ flex: 1 }}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '4px' }}>
                                <span className="mono" style={{ fontWeight: 600 }}>{device.did}</span>
                                <span style={{ fontSize: '0.65rem', padding: '2px 8px', borderRadius: '10px', background: device.status === 'ACTIVE' ? 'rgba(0, 255, 156, 0.1)' : 'rgba(255, 255, 255, 0.05)', color: device.status === 'ACTIVE' ? 'var(--brand-primary)' : 'var(--text-tertiary)', fontWeight: 700 }}>
                                    {device.status}
                                </span>
                            </div>
                            <div style={{ display: 'flex', gap: '16px', color: 'var(--text-tertiary)', fontSize: '0.75rem' }}>
                                <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}><Shield size={12} /> {device.type}</span>
                                <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}><MapPin size={12} /> {device.location}</span>
                            </div>
                        </div>

                        <div style={{ textAlign: 'right' }}>
                            <div style={{ color: 'var(--text-tertiary)', fontSize: '0.65rem', fontWeight: 700, marginBottom: '4px' }}>LAST ACTIVITY</div>
                            <div style={{ fontSize: '0.875rem' }}>2 mins ago</div>
                        </div>

                        <button style={{ padding: '8px 16px', borderRadius: '8px', border: '1px solid var(--border-color)', fontSize: '0.75rem', fontWeight: 600 }}>
                            MANAGE
                        </button>
                    </div>
                ))}
            </div>
        </div>
    );
};

export default DeviceRegistry;
