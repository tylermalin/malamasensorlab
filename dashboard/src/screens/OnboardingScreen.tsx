import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Cpu, Globe, Factory, Zap, CheckCircle2, Loader2, Link2, ShieldCheck } from 'lucide-react';

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
            const mockDid = `did:malama:sensor:${Math.random().toString(36).substring(2, 11)}`;
            setDidResult({
                did: mockDid,
                doc: { id: mockDid, created: new Date().toISOString() }
            });
            setStep(3);
        }, 3000);
    };

    return (
        <div style={{ maxWidth: '840px' }}>
            <div style={{ display: 'flex', gap: '12px', marginBottom: '40px' }}>
                {[1, 2, 3].map((s) => (
                    <div
                        key={s}
                        style={{
                            height: '6px',
                            flex: 1,
                            background: s <= step ? 'var(--brand-primary)' : 'rgba(255,255,255,0.05)',
                            borderRadius: '3px',
                            transition: 'all 0.6s cubic-bezier(0.4, 0, 0.2, 1)',
                            boxShadow: s === step ? '0 0 12px var(--brand-primary)' : 'none'
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
                        style={{ padding: '48px', borderRadius: '32px' }}
                    >
                        <h3 style={{ fontSize: '2rem', fontWeight: 800, marginBottom: '12px', letterSpacing: '-0.02em' }}>Sensor Metadata</h3>
                        <p style={{ color: 'var(--text-secondary)', marginBottom: '40px', fontWeight: 500 }}>Define the physical characteristics of the environmental sensor.</p>

                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '32px' }}>
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
                                marginTop: '48px',
                                width: '100%',
                                padding: '20px',
                                background: 'var(--brand-primary)',
                                color: 'black',
                                fontWeight: 800,
                                fontSize: '1rem',
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                gap: '12px',
                                transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)'
                            }}
                        >
                            {isSimulating ? <Loader2 className="animate-spin" size={20} /> : <Zap size={20} />}
                            {isSimulating ? 'ESTABLISHING_HANDSHAKE...' : 'CONNECT_TO_DEVICE'}
                        </button>
                    </motion.div>
                )}

                {step === 2 && (
                    <motion.div
                        key="step2"
                        initial={{ opacity: 0, scale: 0.95 }}
                        animate={{ opacity: 1, scale: 1 }}
                        className="glass"
                        style={{ padding: '64px 48px', borderRadius: '32px', textAlign: 'center' }}
                    >
                        <div style={{
                            width: '96px', height: '96px', borderRadius: '24px', background: 'rgba(0, 255, 156, 0.05)',
                            border: '1px solid rgba(0, 255, 156, 0.1)',
                            display: 'flex', alignItems: 'center', justifyContent: 'center', margin: '0 auto 32px'
                        }}>
                            <Link2 size={48} color="var(--brand-primary)" />
                        </div>
                        <h3 style={{ fontSize: '2rem', fontWeight: 800, marginBottom: '16px', letterSpacing: '-0.02em' }}>Hardware Handshake Complete</h3>
                        <p style={{ color: 'var(--text-secondary)', marginBottom: '40px', maxWidth: '440px', margin: '0 auto 40px', lineHeight: '1.6', fontWeight: 500 }}>
                            Connection established via WebSerial. Ready to generate unique cryptographic identity and anchor to the Mālama Protocol.
                        </p>

                        <div className="mono" style={{ padding: '24px', background: 'rgba(0,0,0,0.3)', borderRadius: '16px', marginBottom: '48px', fontSize: '0.9rem', textAlign: 'left', border: '1px solid var(--border-color)' }}>
                            <div style={{ opacity: 0.5, marginBottom: '8px', fontSize: '0.75rem', fontWeight: 700 }}>// DEVICE_CHALLENGE</div>
                            <div style={{ color: 'var(--brand-secondary)', fontWeight: 600 }}>0x7D2A9F_HANDSHAKE_VERIFIED</div>
                        </div>

                        <button
                            onClick={handleBirth}
                            disabled={isSimulating}
                            className="brutalist-border"
                            style={{
                                width: '100%',
                                padding: '20px',
                                background: 'var(--brand-primary)',
                                color: 'black',
                                fontWeight: 800,
                                fontSize: '1rem',
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                gap: '12px'
                            }}
                        >
                            {isSimulating ? <Loader2 className="animate-spin" size={20} /> : <ShieldCheck size={20} />}
                            {isSimulating ? 'GENERATING_DID_KEYS...' : 'BIRTH_DEVICE'}
                        </button>
                    </motion.div>
                )}

                {step === 3 && (
                    <motion.div
                        key="step3"
                        initial={{ opacity: 0, y: 30 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="glass"
                        style={{ padding: '56px', borderRadius: '32px' }}
                    >
                        <div style={{ display: 'flex', alignItems: 'center', gap: '24px', marginBottom: '48px' }}>
                            <div style={{
                                width: '72px',
                                height: '72px',
                                borderRadius: '50%',
                                background: 'rgba(0, 255, 156, 0.1)',
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                border: '2px solid var(--brand-primary)',
                                boxShadow: '0 0 20px rgba(0, 255, 156, 0.2)'
                            }}>
                                <CheckCircle2 size={40} color="var(--brand-primary)" />
                            </div>
                            <div>
                                <h3 style={{ fontSize: '2.25rem', fontWeight: 800, letterSpacing: '-0.02em' }}>Device Born Successfully</h3>
                                <p style={{ color: 'var(--brand-primary)', fontWeight: 700, fontSize: '0.875rem' }}>CRYPTOGRAPHIC IDENTITY ANCHORED TO PROTOCOL</p>
                            </div>
                        </div>

                        <div style={{ display: 'grid', gap: '16px' }}>
                            <ResultRow label="DID ADDRESS" value={didResult.did} mono />
                            <ResultRow label="CREATION_TIMESTAMP" value={didResult.doc.created} mono />
                            <ResultRow label="STATUS" value="MASTER_NODE_REGISTERED" color="var(--brand-primary)" />
                        </div>

                        <button
                            onClick={() => setStep(1)}
                            style={{
                                marginTop: '48px',
                                width: '100%',
                                padding: '20px',
                                background: 'rgba(255,255,255,0.03)',
                                border: '1px solid var(--border-color)',
                                color: 'white',
                                fontWeight: 700,
                                borderRadius: '16px',
                                transition: 'all 0.2s'
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
        <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-tertiary)', fontSize: '0.7rem', fontWeight: 900, marginBottom: '12px', letterSpacing: '0.1em' }}>
            <Icon size={14} /> {label}
        </label>
        <input
            type="text"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className="mono"
            style={{
                width: '100%', background: 'rgba(0,0,0,0.2)', border: '1px solid var(--border-color)',
                borderRadius: '12px', padding: '16px 20px', color: 'white', fontSize: '0.9rem',
                outline: 'none', transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
                fontWeight: 500
            }}
            onFocus={(e) => {
                e.target.style.borderColor = 'var(--brand-primary)';
                e.target.style.background = 'rgba(0, 255, 156, 0.02)';
            }}
            onBlur={(e) => {
                e.target.style.borderColor = 'var(--border-color)';
                e.target.style.background = 'rgba(0,0,0,0.2)';
            }}
        />
    </div>
);

const ResultRow = ({ label, value, mono, color }: { label: string, value: string, mono?: boolean, color?: string }) => (
    <div style={{ padding: '20px 24px', background: 'rgba(0,0,0,0.2)', borderRadius: '16px', border: '1px solid var(--border-color)' }}>
        <div style={{ fontSize: '0.65rem', fontWeight: 900, color: 'var(--text-tertiary)', marginBottom: '8px', letterSpacing: '0.1em' }}>{label}</div>
        <div className={mono ? 'mono' : ''} style={{ fontSize: '0.95rem', wordBreak: 'break-all', color: color || 'white', fontWeight: 600 }}>{value}</div>
    </div>
);

export default OnboardingScreen;
