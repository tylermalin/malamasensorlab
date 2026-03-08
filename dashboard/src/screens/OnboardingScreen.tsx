import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Cpu, Globe, Factory, Zap, CheckCircle2, Loader2, Link2, ShieldCheck as ShieldIcon } from 'lucide-react';

const OnboardingScreen: React.FC = () => {
    const [step, setStep] = useState(1);
    const [formData, setFormData] = useState({
        type: 'Air Quality (PM2.5)',
        manufacturer: 'Malama Labs',
        lat: '21.3069',
        lon: '-157.8583'
    });
    const [isSimulating, setIsSimulating] = useState(false);
    const [didResult, setDidResult] = useState<any>(null);

    const handleSimulate = () => {
        setIsSimulating(true);
        setTimeout(() => {
            setIsSimulating(false);
            setStep(2);
        }, 2500);
    };

    const handleBirth = () => {
        setIsSimulating(true);
        setTimeout(() => {
            setIsSimulating(false);
            const mockDid = `did:cardano:sensor:${Math.random().toString(36).substring(2, 15)}`;
            setDidResult({
                did: mockDid,
                doc: { id: mockDid, created: new Date().toISOString() }
            });
            setStep(3);
        }, 3000);
    };

    return (
        <div style={{ maxWidth: '800px' }}>
            <div style={{ display: 'flex', gap: '8px', marginBottom: '32px' }}>
                {[1, 2, 3].map((s) => (
                    <div
                        key={s}
                        style={{
                            height: '4px',
                            flex: 1,
                            background: s <= step ? 'var(--brand-primary)' : 'var(--bg-tertiary)',
                            borderRadius: '2px',
                            transition: 'background 0.5s ease'
                        }}
                    />
                ))}
            </div>

            <AnimatePresence mode="wait">
                {step === 1 && (
                    <motion.div
                        key="step1"
                        initial={{ opacity: 0, x: 20 }}
                        animate={{ opacity: 1, x: 0 }}
                        exit={{ opacity: 0, x: -20 }}
                        className="glass"
                        style={{ padding: '40px', borderRadius: '24px' }}
                    >
                        <h3 style={{ fontSize: '1.5rem', fontWeight: 700, marginBottom: '8px' }}>Sensor Metadata</h3>
                        <p style={{ color: 'var(--text-tertiary)', marginBottom: '32px' }}>Define the physical characteristics of the environmental sensor.</p>

                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '24px' }}>
                            <InputGroup label="SENSOR TYPE" icon={Cpu} value={formData.type} onChange={(v) => setFormData({ ...formData, type: v })} />
                            <InputGroup label="MANUFACTURER" icon={Factory} value={formData.manufacturer} onChange={(v) => setFormData({ ...formData, manufacturer: v })} />
                            <InputGroup label="LATITUDE" icon={Globe} value={formData.lat} onChange={(v) => setFormData({ ...formData, lat: v })} />
                            <InputGroup label="LONGITUDE" icon={Globe} value={formData.lon} onChange={(v) => setFormData({ ...formData, lon: v })} />
                        </div>

                        <button
                            onClick={handleSimulate}
                            disabled={isSimulating}
                            className="brutalist-border"
                            style={{
                                marginTop: '40px',
                                width: '100%',
                                padding: '16px',
                                background: 'var(--brand-primary)',
                                color: 'black',
                                fontWeight: 700,
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                gap: '12px'
                            }}
                        >
                            {isSimulating ? <Loader2 className="animate-spin" /> : <Zap size={18} />}
                            {isSimulating ? 'ESTABLISHING HANDSHAKE...' : 'CONNECT TO DEVICE'}
                        </button>
                    </motion.div>
                )}

                {step === 2 && (
                    <motion.div
                        key="step2"
                        initial={{ opacity: 0, scale: 0.95 }}
                        animate={{ opacity: 1, scale: 1 }}
                        className="glass"
                        style={{ padding: '40px', borderRadius: '24px', textAlign: 'center' }}
                    >
                        <div style={{
                            width: '80px', height: '80px', borderRadius: '50%', background: 'rgba(0, 255, 156, 0.1)',
                            display: 'flex', alignItems: 'center', justifyContent: 'center', margin: '0 auto 24px'
                        }}>
                            <Link2 size={40} color="var(--brand-primary)" />
                        </div>
                        <h3 style={{ fontSize: '1.5rem', fontWeight: 700, marginBottom: '12px' }}>Hardware Handshake Complete</h3>
                        <p style={{ color: 'var(--text-secondary)', marginBottom: '32px', maxWidth: '400px', margin: '0 auto 32px' }}>
                            Connection established via WebSerial. Ready to generate unique cryptographic identity and store on-device.
                        </p>

                        <div className="mono" style={{ padding: '20px', background: 'var(--bg-primary)', borderRadius: '12px', marginBottom: '40px', fontSize: '0.875rem', textAlign: 'left', border: '1px solid var(--border-color)' }}>
                            <div style={{ opacity: 0.5, marginBottom: '4px' }}>// DEVICE_CHALLENGE</div>
                            <div style={{ color: 'var(--brand-secondary)' }}>0x7d2a9f...e4b1c</div>
                        </div>

                        <button
                            onClick={handleBirth}
                            disabled={isSimulating}
                            className="brutalist-border"
                            style={{
                                width: '100%',
                                padding: '16px',
                                background: 'var(--brand-primary)',
                                color: 'black',
                                fontWeight: 700,
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                gap: '12px'
                            }}
                        >
                            {isSimulating ? <Loader2 className="animate-spin" /> : <ShieldCheck size={18} />}
                            {isSimulating ? 'GENERATING DID & KEYS...' : 'BIRTH DEVICE'}
                        </button>
                    </motion.div>
                )}

                {step === 3 && (
                    <motion.div
                        key="step3"
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="glass"
                        style={{ padding: '40px', borderRadius: '24px' }}
                    >
                        <div style={{ display: 'flex', alignItems: 'center', gap: '16px', marginBottom: '32px' }}>
                            <CheckCircle2 size={48} color="var(--brand-primary)" />
                            <div>
                                <h3 style={{ fontSize: '1.5rem', fontWeight: 700 }}>Device Born Successfully</h3>
                                <p style={{ color: 'var(--text-tertiary)' }}>Cryptographic identity anchored to Cardano Testnet.</p>
                            </div>
                        </div>

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                            <ResultRow label="DID ADDRESS" value={didResult.did} mono />
                            <ResultRow label="CREATED" value={didResult.doc.created} />
                            <ResultRow label="STATUS" value="ACTIVE / REGISTERED" color="var(--brand-primary)" />
                        </div>

                        <button
                            onClick={() => setStep(1)}
                            style={{
                                marginTop: '40px',
                                width: '100%',
                                padding: '16px',
                                background: 'var(--bg-tertiary)',
                                color: 'white',
                                fontWeight: 600,
                                borderRadius: '12px'
                            }}
                        >
                            ONBOARD ANOTHER DEVICE
                        </button>
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
};

const InputGroup = ({ label, icon: Icon, value, onChange }: { label: string, icon: any, value: string, onChange: (v: string) => void }) => (
    <div>
        <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-tertiary)', fontSize: '0.65rem', fontWeight: 800, marginBottom: '8px', letterSpacing: '0.05em' }}>
            <Icon size={12} /> {label}
        </label>
        <input
            type="text"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            style={{
                width: '100%', background: 'var(--bg-secondary)', border: '1px solid var(--border-color)',
                borderRadius: '8px', padding: '12px 16px', color: 'white', fontSize: '0.875rem',
                outline: 'none', transition: 'border-color 0.2s'
            }}
            onFocus={(e) => e.target.style.borderColor = 'var(--brand-primary)'}
            onBlur={(e) => e.target.style.borderColor = 'var(--border-color)'}
        />
    </div>
);

const ResultRow = ({ label, value, mono, color }: { label: string, value: string, mono?: boolean, color?: string }) => (
    <div style={{ padding: '16px', background: 'var(--bg-primary)', borderRadius: '12px', border: '1px solid var(--border-color)' }}>
        <div style={{ fontSize: '0.65rem', fontWeight: 800, color: 'var(--text-tertiary)', marginBottom: '4px' }}>{label}</div>
        <div className={mono ? 'mono' : ''} style={{ fontSize: '0.875rem', wordBreak: 'break-all', color: color || 'white' }}>{value}</div>
    </div>
);

export default OnboardingScreen;
const ShieldCheck = ({ size, style, color }: any) => <ShieldIcon size={size} style={style} color={color} />; // Fixed missing icon
