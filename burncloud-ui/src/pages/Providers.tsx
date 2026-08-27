import React, { useState } from 'react';
import { Button, Card, Badge, Input } from '@/components/ui';
import { Plus, ServerCrash, KeyRound, Activity, DollarSign, Wifi, Check, AlertTriangle, AlertCircle } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';

interface ProviderItem {
  name: string;
  status: 'Connected' | 'Degraded' | 'Offline';
  keyHealth: string;
  usage: number;
  spend: number;
  incident: string;
  routes: number;
  latency?: number;
}

const INITIAL_PROVIDERS: ProviderItem[] = [
  { name: 'OpenAI', status: 'Connected', keyHealth: 'Valid', usage: 68, spend: 2526, incident: 'None', routes: 14, latency: 124 },
  { name: 'Anthropic', status: 'Degraded', keyHealth: 'Valid', usage: 91, spend: 34455, incident: 'Timeout spike 42 mins ago', routes: 9, latency: 265 },
  { name: 'Google AI', status: 'Connected', keyHealth: 'Valid', usage: 42, spend: 283, incident: 'None', routes: 6, latency: 110 },
  { name: 'DeepSeek', status: 'Connected', keyHealth: 'Valid', usage: 85, spend: 1278, incident: 'None', routes: 8, latency: 450 },
];

export function Providers() {
  const [providers, setProviders] = useState<ProviderItem[]>(INITIAL_PROVIDERS);
  const [showModal, setShowModal] = useState(false);
  const [testState, setTestState] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [isAuditing, setIsAuditing] = useState(false);
  const [auditTarget, setAuditTarget] = useState<string | null>(null);

  // Form states
  const [selectedProviderName, setSelectedProviderName] = useState('OpenAI');
  const [apiKey, setApiKey] = useState('');
  const [baseUrl, setBaseUrl] = useState('');

  const handleTest = () => {
    if (!apiKey) {
      setTestState('error');
      return;
    }
    setTestState('loading');
    setTimeout(() => {
      setTestState('success');
    }, 1200);
  };

  const handleAddProvider = (e: React.FormEvent) => {
    e.preventDefault();
    if (testState !== 'success') return;

    const newProvider: ProviderItem = {
      name: selectedProviderName,
      status: 'Connected',
      keyHealth: 'Valid',
      usage: 0,
      spend: 0,
      incident: 'None',
      routes: 1,
      latency: Math.floor(Math.random() * 200) + 80,
    };

    setProviders((prev) => {
      if (prev.some((p) => p.name === selectedProviderName)) {
        return prev.map((p) => p.name === selectedProviderName ? { ...p, status: 'Connected', keyHealth: 'Valid' } : p);
      }
      return [...prev, newProvider];
    });

    setShowModal(false);
    setTestState('idle');
    setApiKey('');
    setBaseUrl('');
  };

  const runLatencyAudit = () => {
    setIsAuditing(true);
    let index = 0;

    const runNextPing = () => {
      if (index < providers.length) {
        const target = providers[index].name;
        setAuditTarget(target);

        setTimeout(() => {
          setProviders((prev) =>
            prev.map((p) => {
              if (p.name === target) {
                // Generate a randomized live latency reading
                const variance = Math.floor(Math.random() * 40) - 20;
                const base = target === 'Google AI' ? 95 : target === 'OpenAI' ? 115 : target === 'Anthropic' ? 240 : 420;
                const finalLatency = Math.max(50, base + variance);
                return {
                  ...p,
                  latency: finalLatency,
                  status: finalLatency > 300 ? 'Degraded' : 'Connected',
                };
              }
              return p;
            })
          );
          index++;
          runNextPing();
        }, 600);
      } else {
        setIsAuditing(false);
        setAuditTarget(null);
      }
    };

    runNextPing();
  };

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Providers</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Manage API connections to external foundation model providers.</p>
        </div>
        <div className="flex items-center gap-3">
          <Button 
            variant="secondary" 
            onClick={runLatencyAudit} 
            disabled={isAuditing}
            className="gap-2 text-[13px] relative overflow-hidden"
          >
            <Wifi className={cn("w-4 h-4", isAuditing && "animate-pulse text-orange-500")} />
            {isAuditing ? `Auditing ${auditTarget}...` : 'Run Latency Audit'}
          </Button>
          <Button onClick={() => setShowModal(true)} className="gap-2 text-[13px]"><Plus className="w-4 h-4" /> Add Provider</Button>
        </div>
      </div>

      {/* Grid Layout of Providers */}
      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
        {providers.map((provider, i) => (
          <motion.div
            key={provider.name}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.05 }}
            layout
          >
            <Card className="p-6 h-full flex flex-col hover:shadow-md transition-all duration-300 group border-gray-200/60 hover:border-gray-300 relative overflow-hidden">
              {auditTarget === provider.name && (
                <div className="absolute inset-x-0 top-0 h-[2px] bg-gradient-to-r from-transparent via-orange-500 to-transparent animate-pulse" />
              )}
              
              <div className="flex items-center justify-between mb-6">
                <div className="flex items-center gap-2.5">
                  <h3 className="font-semibold text-lg text-gray-900">{provider.name}</h3>
                  {auditTarget === provider.name && (
                    <span className="flex h-2 w-2 relative">
                      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-orange-400 opacity-75"></span>
                      <span className="relative inline-flex rounded-full h-2 w-2 bg-orange-500"></span>
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  {provider.latency && (
                    <span className={cn(
                      "font-mono text-xs font-semibold px-2 py-0.5 rounded-md",
                      provider.latency < 150 ? "bg-green-50 text-green-700 border border-green-100" :
                      provider.latency < 300 ? "bg-amber-50 text-amber-700 border border-amber-100" :
                      "bg-red-50 text-red-700 border border-red-100"
                    )}>
                      {provider.latency}ms
                    </span>
                  )}
                  <Badge variant={provider.status === 'Connected' ? 'success' : provider.status === 'Degraded' ? 'warning' : 'error'}>
                    {provider.status}
                  </Badge>
                </div>
              </div>

              <div className="space-y-4 flex-1">
                <div className="flex items-center justify-between text-[13px]">
                  <span className="text-gray-500 flex items-center gap-2"><KeyRound className="w-[14px] h-[14px]" /> API Key Health</span>
                  <span className="font-medium text-gray-900 flex items-center gap-1.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-green-500" />
                    {provider.keyHealth}
                  </span>
                </div>
                <div className="flex items-center justify-between text-[13px]">
                  <span className="text-gray-500 flex items-center gap-2"><Activity className="w-[14px] h-[14px]" /> Rate Limit</span>
                  <div className="flex items-center gap-2">
                    <div className="w-16 h-[4px] bg-gray-100 rounded-full overflow-hidden">
                      <div className={cn(
                        "h-full rounded-full transition-all duration-500",
                        provider.usage > 80 ? 'bg-amber-500' : 'bg-green-500'
                      )} style={{ width: `${provider.usage}%` }} />
                    </div>
                    <span className="font-medium text-gray-900 tabular-nums w-8 text-right">{provider.usage}%</span>
                  </div>
                </div>
                <div className="flex items-center justify-between text-[13px]">
                  <span className="text-gray-500 flex items-center gap-2"><DollarSign className="w-[14px] h-[14px]" /> Monthly Spend</span>
                  <span className="font-medium text-gray-900 tabular-nums">${provider.spend.toLocaleString()}</span>
                </div>
                <div className="flex items-center justify-between text-[13px]">
                  <span className="text-gray-500 flex items-center gap-2"><ServerCrash className="w-[14px] h-[14px]" /> Last Incident</span>
                  <span className={cn(
                    "font-medium text-xs truncate max-w-[150px]",
                    provider.incident !== 'None' ? 'text-amber-600 font-semibold' : 'text-gray-500'
                  )} title={provider.incident}>
                    {provider.incident}
                  </span>
                </div>
              </div>

              <div className="mt-6 pt-4 border-t border-gray-100 flex items-center justify-between">
                <span className="text-[13px] text-gray-500">{provider.routes} enabled routes</span>
                <Button variant="secondary" size="sm" className="opacity-0 group-hover:opacity-100 transition-opacity">Configure</Button>
              </div>
            </Card>
          </motion.div>
        ))}
      </div>

      {/* Add Provider Modal */}
      <AnimatePresence>
        {showModal && (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            <motion.div 
              initial={{ opacity: 0 }} 
              animate={{ opacity: 1 }} 
              exit={{ opacity: 0 }} 
              className="absolute inset-0 bg-gray-900/20 backdrop-blur-[2px]"
              onClick={() => setShowModal(false)}
            />
            <motion.div 
              initial={{ opacity: 0, scale: 0.95, y: 10 }} 
              animate={{ opacity: 1, scale: 1, y: 0 }} 
              exit={{ opacity: 0, scale: 0.95, y: 10 }} 
              transition={{ type: 'spring', damping: 25, stiffness: 300 }}
              className="relative bg-white/95 backdrop-blur-xl rounded-[24px] shadow-[0_24px_60px_-12px_rgba(0,0,0,0.15)] w-full max-w-md overflow-hidden border border-gray-200/60"
            >
              <div className="px-7 py-5 border-b border-gray-100/80 bg-white/50">
                <h3 className="text-[17px] font-semibold text-gray-900 tracking-tight">Connect a provider</h3>
              </div>
              
              <form onSubmit={handleAddProvider}>
                <div className="p-7 space-y-5">
                  <div className="space-y-2">
                    <label className="text-[13px] font-medium text-gray-900">Provider</label>
                    <select 
                      value={selectedProviderName}
                      onChange={(e) => {
                        setSelectedProviderName(e.target.value);
                        setTestState('idle');
                      }}
                      className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
                    >
                      <option value="OpenAI">OpenAI</option>
                      <option value="Anthropic">Anthropic</option>
                      <option value="Google AI">Google AI</option>
                      <option value="DeepSeek">DeepSeek</option>
                      <option value="Cohere">Cohere</option>
                      <option value="Mistral">Mistral AI</option>
                    </select>
                  </div>
                  <div className="space-y-2">
                    <label className="text-[13px] font-medium text-gray-900">API Base URL <span className="text-gray-400 font-normal ml-1">(Optional)</span></label>
                    <Input 
                      placeholder="e.g. https://api.openai.com/v1" 
                      value={baseUrl}
                      onChange={(e) => setBaseUrl(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <label className="text-[13px] font-medium text-gray-900 flex items-center justify-between">
                      API Key
                      {apiKey && <span className="text-[11px] text-green-600 font-medium">Value entered</span>}
                    </label>
                    <Input 
                      type="password" 
                      required
                      placeholder="Enter your secret credentials (e.g. sk-...)" 
                      value={apiKey}
                      onChange={(e) => {
                        setApiKey(e.target.value);
                        if (testState === 'error') setTestState('idle');
                      }}
                    />
                    <p className="text-[12px] text-gray-500 mt-1.5 leading-relaxed">Keys are securely parsed, encrypted at rest, and never sent to client-side logs.</p>
                  </div>
                  
                  {testState !== 'idle' && (
                    <div className={cn(
                      "p-3.5 rounded-[14px] text-[13px] font-medium border flex items-center gap-2",
                      testState === 'loading' ? 'bg-gray-50/50 text-gray-600 border-gray-200/60' : 
                      testState === 'success' ? 'bg-green-50/50 text-green-700 border-green-200/60' : 
                      'bg-red-50/50 text-red-700 border-red-200/60'
                    )}>
                      {testState === 'loading' && (
                        <>
                          <div className="w-4 h-4 border-2 border-gray-400 border-t-gray-800 rounded-full animate-spin flex-shrink-0" />
                          <span>Testing secure connection...</span>
                        </>
                      )}
                      {testState === 'success' && (
                        <>
                          <Check className="w-4 h-4 text-green-600 flex-shrink-0" />
                          <span>Connection verified. Latency: {Math.floor(Math.random() * 120) + 70}ms.</span>
                        </>
                      )}
                      {testState === 'error' && (
                        <>
                          <AlertCircle className="w-4 h-4 text-red-600 flex-shrink-0" />
                          <span>Invalid API key or network timeout. Check values.</span>
                        </>
                      )}
                    </div>
                  )}
                </div>
                
                <div className="px-7 py-5 bg-gray-50/50 flex items-center justify-between border-t border-gray-100/80">
                  <Button type="button" variant="ghost" onClick={handleTest} disabled={testState === 'loading'} className="text-[13px]">
                    {testState === 'loading' ? 'Testing...' : 'Test Connection'}
                  </Button>
                  <div className="flex items-center gap-2">
                    <Button type="button" variant="secondary" onClick={() => setShowModal(false)} className="text-[13px]">Cancel</Button>
                    <Button type="submit" disabled={testState !== 'success'} className="text-[13px]">Save Provider</Button>
                  </div>
                </div>
              </form>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}

