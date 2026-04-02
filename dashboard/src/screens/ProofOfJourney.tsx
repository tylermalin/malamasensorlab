import React from 'react';
import { motion } from 'framer-motion';
import {
    Search,
    ExternalLink,
    MapPin,
    ShieldCheck
} from 'lucide-react';

const mockProofs = [
    { id: 'MALAMA-CERT-8812', sensor: 'did:malama:sensor:8b3...', location: 'Nairobi, Kenya', timestamp: '2026-03-08 14:22', tokens: '42 LCO2', status: 'VERIFIED' },
    { id: 'MALAMA-CERT-8813', sensor: 'did:malama:sensor:f92...', location: 'Bali, Indonesia', timestamp: '2026-03-08 14:15', tokens: '120 VCO2', status: 'VERIFIED' },
    { id: 'MALAMA-CERT-8814', sensor: 'did:malama:sensor:a44...', location: 'Amazonas, Brazil', timestamp: '2026-03-08 13:58', tokens: '350 VCO2', status: 'VERIFIED' },
];

const ProofOfJourney: React.FC = () => {
    return (
        <div style={{ padding: '24px' }}>
            <div style={{ marginBottom: '40px', display: 'flex', gap: '16px' }}>
                <div className="glass" style={{ flex: 1, padding: '12px 24px', borderRadius: '12px', display: 'flex', alignItems: 'center', gap: '12px' }}>
                    <Search size={18} color="var(--text-tertiary)" />
                    <input
                        placeholder="Search Cert ID, Sensor DID, or Transaction..."
                        style={{ background: 'none', border: 'none', color: 'var(--text-primary)', width: '100%', fontSize: '0.875rem', outline: 'none' }}
                    />
                </div>
                <button className="brutalist-border" style={{ padding: '12px 32px', background: 'var(--brand-primary)', color: 'black', fontWeight: 700 }}>
                    AUDIT_SCAN
                </button>
            </div>

            <div style={{ display: 'grid', gap: '16px' }}>
                {mockProofs.map((proof, index) => (
                    <motion.div
                        key={proof.id}
                        initial={{ opacity: 0, x: -20 }}
                        animate={{ opacity: 1, x: 0 }}
                        transition={{ delay: index * 0.1 }}
                        className="glass"
                        style={{ padding: '24px', borderRadius: '16px', display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr auto', alignItems: 'center', gap: '24px' }}
                    >
                        <div>
                            <div className="mono" style={{ fontSize: '0.7rem', color: 'var(--brand-primary)', marginBottom: '4px' }}>{proof.id}</div>
                            <div style={{ fontSize: '0.875rem', fontWeight: 700 }}>{proof.tokens}</div>
                        </div>

                        <div>
                            <div style={{ fontSize: '0.7rem', color: 'var(--text-tertiary)', fontWeight: 700, textTransform: 'uppercase', marginBottom: '4px' }}>Sensor</div>
                            <div className="mono" style={{ fontSize: '0.75rem' }}>{proof.sensor}</div>
                        </div>

                        <div>
                            <div style={{ fontSize: '0.7rem', color: 'var(--text-tertiary)', fontWeight: 700, textTransform: 'uppercase', marginBottom: '4px' }}>Location</div>
                            <div style={{ fontSize: '0.75rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '6px' }}>
                                <MapPin size={12} color="var(--brand-secondary)" />
                                {proof.location}
                            </div>
                        </div>

                        <div>
                            <div style={{ fontSize: '0.7rem', color: 'var(--text-tertiary)', fontWeight: 700, textTransform: 'uppercase', marginBottom: '4px' }}>Status</div>
                            <div style={{ fontSize: '0.7rem', fontWeight: 800, color: 'var(--brand-primary)', background: 'rgba(0, 255, 156, 0.1)', padding: '4px 8px', borderRadius: '4px', display: 'inline-block' }}>
                                {proof.status}
                            </div>
                        </div>

                        <button style={{ color: 'var(--text-tertiary)', transition: 'color 0.2s' }}>
                            <ExternalLink size={18} />
                        </button>
                    </motion.div>
                ))}
            </div>

            {/* Verification Logic Simulation */}
            <div className="glass" style={{ marginTop: '48px', padding: '32px', borderRadius: '24px', border: '1px dashed var(--brand-secondary)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '16px', marginBottom: '24px' }}>
                    <ShieldCheck size={32} color="var(--brand-secondary)" />
                    <h4 style={{ fontWeight: 800 }}>LIVE_AUDIT_VERIFIER</h4>
                </div>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '24px' }}>
                    <AuditStep label="Merkle Root" status="MATCH" />
                    <AuditStep label="LSH Fingerprint" status="SECURE" />
                    <AuditStep label="Validator Sigs" status="QUORUM" />
                    <AuditStep label="Chain Anchor" status="VERIFIED" />
                </div>
            </div>
        </div>
    );
};

const AuditStep = ({ label, status }: any) => (
    <div style={{ padding: '16px', background: 'rgba(0,0,0,0.2)', borderRadius: '12px' }}>
        <div style={{ fontSize: '0.65rem', color: 'var(--text-tertiary)', fontWeight: 800, textTransform: 'uppercase', marginBottom: '8px' }}>{label}</div>
        <div className="mono" style={{ fontSize: '0.75rem', fontWeight: 700, color: 'var(--brand-secondary)' }}>[{status}]</div>
    </div>
);

export default ProofOfJourney;
