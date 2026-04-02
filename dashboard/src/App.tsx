import React, { useState } from 'react';
import {
  LayoutDashboard,
  Smartphone,
  History,
  Settings,
  PlusCircle,
  Cpu,
  Activity,
  ShieldCheck,
  Zap,
  Globe,
  Tag
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

// Mock Screens
import OnboardingScreen from './screens/OnboardingScreen';
import DeviceRegistry from './screens/DeviceRegistry';
import DataJournal from './screens/DataJournal';
import OdysseyVisualizer from './screens/OdysseyVisualizer';
import ProofOfJourney from './screens/ProofOfJourney';

const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState('onboarding');

  const menuItems = [
    { id: 'dashboard', icon: LayoutDashboard, label: 'Overview' },
    { id: 'onboarding', icon: PlusCircle, label: 'Device Birth' },
    { id: 'registry', icon: Smartphone, label: 'Registry' },
    { id: 'odyssey', icon: Globe, label: 'Data Odyssey' },
    { id: 'marketplace', icon: Tag, label: 'Marketplace' },
    { id: 'journal', icon: History, label: 'Data Journal' },
  ];

  return (
    <div className="dashboard-grid">
      <div className="scanline" />

      {/* Sidebar */}
      <aside className="glass" style={{ zIndex: 10, position: 'relative', display: 'flex', flexDirection: 'column' }}>
        <div style={{ padding: '40px 32px', display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div className="brutalist-border" style={{ padding: '6px', background: 'var(--brand-primary)' }}>
            <Zap size={20} color="black" />
          </div>
          <h1 style={{ fontSize: '1.25rem', fontWeight: 800, letterSpacing: '-0.03em' }}>
            MĀLAMA <span style={{ color: 'var(--text-tertiary)' }}>CORE</span>
          </h1>
        </div>

        <nav style={{ flex: 1, padding: '0 16px' }}>
          {menuItems.map((item) => (
            <button
              key={item.id}
              onClick={() => setActiveTab(item.id)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '14px',
                width: '100%',
                padding: '12px 20px',
                borderRadius: '12px',
                marginBottom: '4px',
                transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
                background: activeTab === item.id ? 'rgba(0, 255, 156, 0.05)' : 'transparent',
                color: activeTab === item.id ? 'var(--brand-primary)' : 'var(--text-secondary)',
                border: '1px solid transparent',
                borderColor: activeTab === item.id ? 'rgba(0, 255, 156, 0.1)' : 'transparent'
              }}
            >
              <item.icon size={18} strokeWidth={activeTab === item.id ? 2.5 : 2} />
              <span style={{ fontWeight: 600, fontSize: '0.875rem' }}>{item.label}</span>
              {activeTab === item.id && (
                <motion.div
                  layoutId="active-indicator"
                  style={{ marginLeft: 'auto', width: '4px', height: '4px', borderRadius: '50%', background: 'var(--brand-primary)', boxShadow: '0 0 8px var(--brand-primary)' }}
                />
              )}
            </button>
          ))}
        </nav>

        <div style={{ padding: '24px', borderTop: '1px solid var(--border-color)', display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div style={{ width: '32px', height: '32px', borderRadius: '50%', background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '10px', fontWeight: 700 }}>
            <span>TM</span>
          </div>
          <div style={{ fontSize: '0.75rem' }}>
            <div style={{ fontWeight: 700 }}>Tyler Malin</div>
            <div style={{ color: 'var(--text-tertiary)', fontWeight: 500 }}>Global Admin</div>
          </div>
          <Settings size={16} style={{ marginLeft: 'auto', color: 'var(--text-tertiary)', cursor: 'pointer' }} />
        </div>
      </aside>

      {/* Main Content */}
      <main className="main-content">
        <header style={{ marginBottom: '64px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <div style={{ color: 'var(--brand-primary)', fontSize: '0.7rem', fontWeight: 800, textTransform: 'uppercase', letterSpacing: '0.2em', marginBottom: '8px', opacity: 0.8 }}>
              MALAMA_Odyssey_v0.1.0
            </div>
            <h2 style={{ fontSize: '2.5rem', fontWeight: 800, letterSpacing: '-0.02em' }}>
              {menuItems.find(i => i.id === activeTab)?.label}
            </h2>
          </div>

          <div className="glass" style={{ padding: '10px 20px', borderRadius: '30px', display: 'flex', alignItems: 'center', gap: '12px' }}>
            <div style={{
              width: '8px',
              height: '8px',
              borderRadius: '50%',
              background: 'var(--brand-primary)',
              boxShadow: '0 0 12px var(--brand-primary)',
              animation: 'grid-glow 2s ease-in-out infinite'
            }} />
            <span className="mono" style={{ fontSize: '0.75rem', fontWeight: 600, color: 'var(--brand-primary)' }}>PROTOCOL: ACTIVE</span>
          </div>
        </header>

        <AnimatePresence mode="wait">
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
          >
            {activeTab === 'onboarding' && <OnboardingScreen />}
            {activeTab === 'registry' && <DeviceRegistry />}
            {activeTab === 'odyssey' && <OdysseyVisualizer />}
            {activeTab === 'marketplace' && <ProofOfJourney />}
            {activeTab === 'journal' && <DataJournal />}
            {activeTab === 'dashboard' && (
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '32px' }}>
                <StatsCard icon={Cpu} label="ACTIVE SENSORS" value="1,024" color="var(--brand-primary)" />
                <StatsCard icon={Activity} label="THROUGHPUT (TPS)" value="24.8" color="var(--brand-secondary)" />
                <StatsCard icon={ShieldCheck} label="PROOFS VERIFIED" value="1.2M" color="var(--brand-tertiary)" />
              </div>
            )}
          </motion.div>
        </AnimatePresence>
      </main>
    </div>
  );
};

const StatsCard = ({ icon: Icon, label, value, color }: any) => (
  <div className="glass" style={{ padding: '32px', borderRadius: '24px', position: 'relative', overflow: 'hidden' }}>
    <div style={{ position: 'absolute', top: '-20px', right: '-20px', opacity: 0.1, transform: 'rotate(-15deg)' }}>
      <Icon size={160} color={color} />
    </div>
    <div style={{ color: 'var(--text-tertiary)', fontSize: '0.75rem', fontWeight: 800, letterSpacing: '0.1em', marginBottom: '16px' }}>
      {label}
    </div>
    <div style={{ fontSize: '2.5rem', fontWeight: 900, color, letterSpacing: '-0.02em' }}>
      {value}
    </div>
  </div>
);

export default App;
