import React, { useState } from 'react';
import { Button, Card, Badge, Drawer, Input } from '@/components/ui';
import { Settings as SettingsIcon, Sliders, ShieldAlert, Radio, Database, RefreshCw, CheckCircle2, AlertTriangle, Send, Lock } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';

export function Settings() {
  const [activeTab, setActiveTab] = useState<'routing' | 'webhooks' | 'retention'>('routing');
  const [isSaving, setIsSaving] = useState(false);
  const [showToast, setShowToast] = useState(false);

  // Routing state
  const [defaultTimeout, setDefaultTimeout] = useState(10000);
  const [maxRetries, setMaxRetries] = useState(3);
  const [defaultRoutingMode, setDefaultRoutingMode] = useState('latency-optimized');

  // Webhook state
  const [webhookUrl, setWebhookUrl] = useState('https://example.invalid/webhooks/operations');
  const [isTestingWebhook, setIsTestingWebhook] = useState(false);
  const [webhookTestStatus, setWebhookTestStatus] = useState<string | null>(null);

  // Retention state
  const [retentionDays, setRetentionDays] = useState('90');
  const [piiAnonymization, setPiiAnonymization] = useState(true);

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    setIsSaving(true);
    setTimeout(() => {
      setIsSaving(false);
      setShowToast(true);
      setTimeout(() => setShowToast(false), 3000);
    }, 1200);
  };

  const handleTestWebhook = () => {
    setIsTestingWebhook(true);
    setWebhookTestStatus(null);
    setTimeout(() => {
      setIsTestingWebhook(false);
      setWebhookTestStatus('✅ Webhook successfully delivered: received HTTP 200 OK from Slack Integration.');
    }, 1500);
  };

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out relative">
      {/* Toast Notification */}
      <AnimatePresence>
        {showToast && (
          <motion.div 
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="fixed top-6 right-6 z-50 bg-gray-900 text-white px-4 py-3 rounded-xl shadow-lg flex items-center gap-2.5 text-xs font-semibold"
          >
            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            Global settings updated and dispatched.
          </motion.div>
        )}
      </AnimatePresence>

      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Platform Settings</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Configure global router timeout limits, setup administrative alerting webhooks, and audit compliance logs.</p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Sidebar tabs selection */}
        <div className="space-y-1 lg:col-span-1">
          {[
            { id: 'routing', label: 'Routing Rules & Timeouts', icon: Sliders },
            { id: 'webhooks', label: 'Alert Alerts & Webhooks', icon: Radio },
            { id: 'retention', label: 'Compliance & Archiving', icon: Database }
          ].map((tab) => {
            const Icon = tab.icon;
            const isSelected = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as any)}
                className={cn(
                  "w-full flex items-center gap-3 px-4 py-3 rounded-xl text-left text-[13px] font-semibold transition-all",
                  isSelected 
                    ? "bg-white text-gray-900 border border-gray-200/60 shadow-sm" 
                    : "text-gray-500 hover:text-gray-900 hover:bg-gray-100/50"
                )}
              >
                <Icon className={cn("w-4 h-4", isSelected ? "text-gray-900" : "text-gray-400")} />
                {tab.label}
              </button>
            );
          })}
        </div>

        {/* Configurations content */}
        <div className="lg:col-span-3">
          <Card className="p-6">
            <form onSubmit={handleSave} className="space-y-6 relative">
              {isSaving && (
                <div className="absolute inset-0 bg-white/75 backdrop-blur-[2px] z-10 flex flex-col items-center justify-center">
                  <div className="w-8 h-8 border-2 border-gray-200 border-t-gray-900 rounded-full animate-spin mb-3"></div>
                  <p className="text-xs font-semibold text-gray-900">Synchronizing system defaults...</p>
                </div>
              )}

              {activeTab === 'routing' && (
                <div className="space-y-6 animate-in fade-in duration-300">
                  <div className="border-b border-gray-100 pb-4">
                    <h3 className="text-sm font-semibold text-gray-900">Routing Failover Configuration</h3>
                    <p className="text-xs text-gray-400 mt-1">Determine how the gateway reacts when provider models timeout or return server errors.</p>
                  </div>

                  <div className="space-y-4">
                    {/* Timeout slider */}
                    <div className="space-y-2">
                      <div className="flex justify-between items-center">
                        <label className="text-xs font-bold text-gray-700">Default Request Timeout Threshold</label>
                        <span className="text-xs font-bold text-gray-900 font-mono">{(defaultTimeout / 1000).toFixed(1)}s</span>
                      </div>
                      <input 
                        type="range" 
                        min="1000" 
                        max="30000" 
                        step="500"
                        value={defaultTimeout} 
                        onChange={(e) => setDefaultTimeout(Number(e.target.value))}
                        className="w-full h-1.5 bg-gray-100 rounded-full appearance-none cursor-pointer accent-gray-900"
                      />
                      <span className="text-[11px] text-gray-400 leading-normal block">
                        Requests taking longer than this will trigger automatic failovers to secondary nodes down the fallback chain.
                      </span>
                    </div>

                    {/* Retry count */}
                    <div className="space-y-2">
                      <div className="flex justify-between items-center">
                        <label className="text-xs font-bold text-gray-700">Max Fallback Retry Count</label>
                        <span className="text-xs font-bold text-gray-900 font-mono">{maxRetries} Retries</span>
                      </div>
                      <input 
                        type="range" 
                        min="1" 
                        max="5" 
                        step="1"
                        value={maxRetries} 
                        onChange={(e) => setMaxRetries(Number(e.target.value))}
                        className="w-full h-1.5 bg-gray-100 rounded-full appearance-none cursor-pointer accent-gray-900"
                      />
                      <span className="text-[11px] text-gray-400 leading-normal block">
                        Maximum number of times the router cascades downstream to different models before issuing a final client timeout error.
                      </span>
                    </div>

                    {/* Default Route Strategy */}
                    <div className="space-y-1.5 pt-2">
                      <label className="text-xs font-bold text-gray-700 block">Default Routing Strategy</label>
                      <select 
                        value={defaultRoutingMode}
                        onChange={(e) => setDefaultRoutingMode(e.target.value)}
                        className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
                      >
                        <option value="latency-optimized">Latency-Optimized (Routinely query fastest available cluster)</option>
                        <option value="cost-optimized">Cost-Optimized (Minimize token cost using fallbacks)</option>
                        <option value="stability-first">Stability-First (Prefer highest-reliability models always)</option>
                      </select>
                    </div>
                  </div>
                </div>
              )}

              {activeTab === 'webhooks' && (
                <div className="space-y-6 animate-in fade-in duration-300">
                  <div className="border-b border-gray-100 pb-4">
                    <h3 className="text-sm font-semibold text-gray-900">Incident Alerting Webhooks</h3>
                    <p className="text-xs text-gray-400 mt-1">Setup hooks to receive immediate notifications regarding rate-limiting triggers, outages, or quota alerts.</p>
                  </div>

                  <div className="space-y-4">
                    <div className="space-y-1.5">
                      <label className="text-xs font-bold text-gray-700 block">HTTP POST Endpoint URL</label>
                      <div className="flex gap-2">
                        <Input 
                          required
                          type="url" 
                          value={webhookUrl}
                          onChange={(e) => setWebhookUrl(e.target.value)}
                          placeholder="e.g. https://example.invalid/webhooks/operations"
                        />
                        <Button 
                          type="button" 
                          variant="secondary" 
                          className="gap-1.5 px-4 h-10 text-xs shrink-0" 
                          onClick={handleTestWebhook}
                          disabled={isTestingWebhook || !webhookUrl}
                        >
                          {isTestingWebhook ? (
                            <>
                              <RefreshCw className="w-3.5 h-3.5 animate-spin" /> Ping...
                            </>
                          ) : (
                            <>
                              <Send className="w-3.5 h-3.5" /> Test Hook
                            </>
                          )}
                        </Button>
                      </div>
                    </div>

                    {webhookTestStatus && (
                      <div className="p-3 bg-gray-50 border border-gray-150 rounded-xl flex items-start gap-2 text-xs text-gray-700">
                        <CheckCircle2 className="w-4 h-4 text-emerald-500 shrink-0 mt-0.5" />
                        <span>{webhookTestStatus}</span>
                      </div>
                    )}

                    <div className="space-y-2 pt-2">
                      <label className="text-xs font-bold text-gray-700 block">Dispatch Trigger Events</label>
                      <div className="space-y-2 text-xs">
                        {[
                          { id: 'ev_failover', label: 'Cascaded Fallback Activations', desc: 'Trigger when primary models fail and traffic overflows to secondary models.' },
                          { id: 'ev_incident', label: 'Full Outage / Timeout Alerts', desc: 'Alert immediately if all options in a route fallback list error out.' },
                          { id: 'ev_budget', label: 'Quota & Budget Limits Thresholds', desc: 'Notify once tenant spend velocities cross monthly alert boundaries.' }
                        ].map((ev) => (
                          <label key={ev.id} className="flex items-start gap-3 p-3 bg-gray-50 hover:bg-gray-100/50 rounded-xl cursor-pointer">
                            <input type="checkbox" defaultChecked className="mt-0.5 rounded text-gray-900 focus:ring-gray-900 border-gray-300" />
                            <div>
                              <span className="font-semibold text-gray-800 block">{ev.label}</span>
                              <span className="text-gray-400 text-[11px] leading-normal">{ev.desc}</span>
                            </div>
                          </label>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {activeTab === 'retention' && (
                <div className="space-y-6 animate-in fade-in duration-300">
                  <div className="border-b border-gray-100 pb-4">
                    <h3 className="text-sm font-semibold text-gray-900">Compliance & Retention Rules</h3>
                    <p className="text-xs text-gray-400 mt-1">Define data security configurations regarding local log caching and client PII scrubbing standards.</p>
                  </div>

                  <div className="space-y-4">
                    <div className="space-y-1.5">
                      <label className="text-xs font-bold text-gray-700 block">Audit Log Longevity Policies</label>
                      <select 
                        value={retentionDays}
                        onChange={(e) => setRetentionDays(e.target.value)}
                        className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
                      >
                        <option value="30">Retain logs for 30 Days</option>
                        <option value="90">Retain logs for 90 Days (Enterprise Standard)</option>
                        <option value="180">Retain logs for 180 Days (PCI Compliance)</option>
                        <option value="365">Retain logs for 1 Year</option>
                      </select>
                      <span className="text-[11px] text-gray-400 leading-normal block">
                        Completed transactions are securely archived. After the retention window closes, metadata and inputs are hard-pruned.
                      </span>
                    </div>

                    <div className="space-y-2 pt-2">
                      <label className="text-xs font-bold text-gray-700 block">PII & Log Anonymization</label>
                      <label className="flex items-start gap-3 p-3 bg-gray-50 border border-gray-150 rounded-xl cursor-pointer">
                        <input 
                          type="checkbox" 
                          checked={piiAnonymization} 
                          onChange={(e) => setPiiAnonymization(e.target.checked)}
                          className="mt-1 rounded text-gray-900 focus:ring-gray-900 border-gray-300" 
                        />
                        <div className="space-y-0.5">
                          <span className="font-semibold text-gray-800 text-xs block">Anonymize Client Payloads in Saved Logs</span>
                          <span className="text-gray-400 text-[11px] leading-relaxed block">
                            When enabled, any customer queries logged are scrubbed of emails, credit cards, and addresses before being written to persistent audit logs.
                          </span>
                        </div>
                      </label>
                    </div>
                  </div>
                </div>
              )}

              <div className="pt-5 border-t border-gray-150 flex items-center justify-end gap-3">
                <Button type="button" variant="secondary" onClick={() => {}} disabled={isSaving}>
                  Restore Defaults
                </Button>
                <Button type="submit" disabled={isSaving}>
                  Commit Configuration Changes
                </Button>
              </div>
            </form>
          </Card>
        </div>
      </div>
    </div>
  );
}
