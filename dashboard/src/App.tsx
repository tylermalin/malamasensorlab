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
  Zap
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

// Mock Screens (Will be broken out later)
import OnboardingScreen from './screens/OnboardingScreen';
import DeviceRegistry from './screens/DeviceRegistry';
import DataJournal from './screens/DataJournal';

const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState('onboarding');

  const menuItems = [
    { id: 'dashboard', icon: LayoutDashboard, label: 'Overview' },
    { id: 'onboarding', icon: PlusCircle, label: 'Device Birth' },
    { id: 'registry', icon: Smartphone, label: 'Registry' },
    { id: 'journal', icon: History, label: 'Data Journal' },
  ];

  return (
    <div className="dashboard-grid">
      {/* Sidebar */}
      <aside className="glass" style={{ borderRight: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column' }}>
        <div style={{ padding: '32px', display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div className="brutalist-border" style={{ padding: '6px', background: 'var(--brand-primary)' }}>
            <Zap size={20} color="black" />
          </div>
          <h1 style={{ fontSize: '1.25rem', fontWeight: 700, letterSpacing: '-0.02em' }}>
            MĀLAMA <span style={{ color: 'var(--text-tertiary)' }}>CORE</span>
          </h1>
        </div>

        <nav style={{ flex: 1, padding: '12px' }}>
          {menuItems.map((item) => (
            <button
              key={item.id}
              onClick={() => setActiveTab(item.id)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
                width: '100%',
                padding: '12px 20px',
                borderRadius: '8px',
                marginBottom: '4px',
                transition: 'all 0.2s ease',
                background: activeTab === item.id ? 'var(--glass-bg)' : 'transparent',
                color: activeTab === item.id ? 'var(--brand-primary)' : 'var(--text-secondary)',
                border: activeTab === item.id ? '1px solid var(--glass-border)' : '1px solid transparent'
              }}
            >
              <item.icon size={20} />
              <span style={{ fontWeight: 500 }}>{item.label}</span>
              {activeTab === item.id && (
                <motion.div
                  layoutId="active-indicator"
                  style={{ marginLeft: 'auto', width: '4px', height: '4px', borderRadius: '50%', background: 'var(--brand-primary)' }}
                />
              )}
            </button>
          ))}
        </nav>

        <div style={{ padding: '24px', borderTop: '1px solid var(--border-color)', display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div style={{ width: '32px', height: '32px', borderRadius: '50%', background: 'var(--bg-tertiary)', display: 'flex', alignItems: 'center', justifyItems: 'center', fontSize: '10px' }}>
            <span style={{ margin: 'auto' }}>TM</span>
          </div>
          <div style={{ fontSize: '0.75rem' }}>
            <div style={{ fontWeight: 600 }}>Tyler Malin</div>
            <div style={{ color: 'var(--text-tertiary)' }}>Mālama Admin</div>
          </div>
          <Settings size={16} style={{ marginLeft: 'auto', color: 'var(--text-tertiary)' }} />
        </div>
      </aside>

      {/* Main Content */}
      <main className="main-content">
        <header style={{ marginBottom: '48px', display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
          <div>
            <div style={{ color: 'var(--brand-primary)', fontSize: '0.75rem', fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.1em', marginBottom: '8px' }}>
              INTERNAL SYSTEM
            </div>
            <h2 style={{ fontSize: '2rem', fontWeight: 800 }}>
              {menuItems.find(i => i.id === activeTab)?.label}
            </h2>
          </div>

          <div className="glass" style={{ padding: '8px 16px', borderRadius: '24px', display: 'flex', alignItems: 'center', gap: '12px' }}>
            <div style={{ width: '8px', height: '8px', borderRadius: '50%', background: 'var(--brand-primary)', boxShadow: '0 0 10px var(--brand-primary)' }} />
            <span className="mono" style={{ fontSize: '0.75rem', fontWeight: 500 }}>CORE_NODE: CONNECTED</span>
          </div>
        </header>

        <AnimatePresence mode="wait">
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.2 }}
          >
            {activeTab === 'onboarding' && <OnboardingScreen />}
            {activeTab === 'registry' && <DeviceRegistry />}
            {activeTab === 'journal' && <DataJournal />}
            {activeTab === 'dashboard' && (
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '24px' }}>
                <StatsCard icon={Cpu} label="ACTIVE SENSORS" value="42" color="var(--brand-primary)" />
                <StatsCard icon={Activity} label="REAL-TIME THROUGHPUT" value="2.4 TPS" color="var(--brand-secondary)" />
                <StatsCard icon={ShieldCheck} label="PROOFS PENDING" value="7" color="var(--brand-tertiary)" />
              </div>
            )}
          </motion.div>
        </AnimatePresence>
      </main>
    </div>
  );
};

const StatsCard = ({ icon: Icon, label, value, color }: any) => (
  <div className="glass" style={{ padding: '24px', borderRadius: '16px', position: 'relative', overflow: 'hidden' }}>
    <div style={{ position: 'absolute', top: '-10px', right: '-10px', opacity: 0.05, transform: 'rotate(-15deg)' }}>
      <Icon size={120} color={color} />
    </div>
    <div style={{ color: 'var(--text-tertiary)', fontSize: '0.75rem', fontWeight: 700, letterSpacing: '0.05em', marginBottom: '12px' }}>
      {label}
    </div>
    <div style={{ fontSize: '2rem', fontWeight: 800, color }}>
      {value}
    </div>
  </div>
);

export default App;
