import React, { useState } from 'react';
import { Button, Card, Badge, Drawer, Input } from '@/components/ui';
import { CreditCard, DollarSign, ArrowUpRight, TrendingUp, Download, Calendar, ShieldAlert, Sliders, CheckCircle2, AlertTriangle } from 'lucide-react';
import { ResponsiveContainer, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, Cell, PieChart, Pie } from 'recharts';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';

export function Billing() {
  const [timeRange, setTimeRange] = useState<'24h' | '7d' | '30d'>('30d');
  const [isAlertDrawerOpen, setIsAlertDrawerOpen] = useState(false);
  const [isSavingAlerts, setIsSavingAlerts] = useState(false);
  const [showAlertToast, setShowAlertToast] = useState(false);

  // Form states for alerts
  const [monthlyHardLimit, setMonthlyHardLimit] = useState(50000);
  const [warningThreshold, setWarningThreshold] = useState(80);
  const [alertWebhook, setAlertWebhook] = useState('https://api.burncloud.com/webhooks/billing');

  const providerBreakdown = [
    { name: 'Anthropic', value: 34455, color: '#8b5cf6' },
    { name: 'OpenAI', value: 2526, color: '#10b981' },
    { name: 'DeepSeek', value: 1278, color: '#3b82f6' },
    { name: 'Google AI', value: 283, color: '#f97316' },
  ];

  const tenantBreakdown = [
    { name: 'ETR Global', spend: 24820, tokens: 19800, rps: 120 },
    { name: 'NovaDesk', spend: 8420, tokens: 7200, rps: 60 },
    { name: 'AeroTech', spend: 3120, tokens: 2400, rps: 80 },
    { name: 'AlphaCorp', spend: 1920, tokens: 1100, rps: 30 },
    { name: 'Internal', spend: 1280, tokens: 950, rps: 150 },
  ];

  const dailyHistory = [
    { day: 'Jul 01', Anthropic: 850, OpenAI: 120, DeepSeek: 40, Google: 10 },
    { day: 'Jul 02', Anthropic: 980, OpenAI: 110, DeepSeek: 50, Google: 15 },
    { day: 'Jul 03', Anthropic: 1120, OpenAI: 90, DeepSeek: 45, Google: 12 },
    { day: 'Jul 04', Anthropic: 1450, OpenAI: 150, DeepSeek: 60, Google: 20 },
    { day: 'Jul 05', Anthropic: 1200, OpenAI: 130, DeepSeek: 80, Google: 18 },
    { day: 'Jul 06', Anthropic: 1650, OpenAI: 140, DeepSeek: 110, Google: 22 },
    { day: 'Jul 07', Anthropic: 1820, OpenAI: 160, DeepSeek: 130, Google: 25 },
  ];

  const handleExportData = () => {
    alert('Billing CSV export generated successfully. Check your browser downloads.');
  };

  const handleSaveAlerts = (e: React.FormEvent) => {
    e.preventDefault();
    setIsSavingAlerts(true);
    setTimeout(() => {
      setIsSavingAlerts(false);
      setIsAlertDrawerOpen(false);
      setShowAlertToast(true);
      setTimeout(() => setShowAlertToast(false), 3000);
    }, 1200);
  };

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out relative">
      {/* Toast Notification */}
      <AnimatePresence>
        {showAlertToast && (
          <motion.div 
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="fixed top-6 right-6 z-50 bg-gray-900 text-white px-4 py-3 rounded-xl shadow-lg flex items-center gap-2.5 text-xs font-semibold"
          >
            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            Billing alert profiles updated successfully.
          </motion.div>
        )}
      </AnimatePresence>

      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Billing & Cost Control</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Inspect cost distributions across LLM providers, track customer budgets, and set up hard spending limits.</p>
        </div>
        <div className="flex items-center gap-3">
          <div className="flex bg-gray-100 rounded-lg p-0.5 border border-gray-200/40">
            {(['24h', '7d', '30d'] as const).map((r) => (
              <button
                key={r}
                onClick={() => setTimeRange(r)}
                className={cn(
                  "px-3 py-1 text-xs font-semibold rounded-md transition-all",
                  timeRange === r 
                    ? "bg-white text-gray-900 shadow-sm" 
                    : "text-gray-500 hover:text-gray-900"
                )}
              >
                {r === '24h' ? '24 Hours' : r === '7d' ? '7 Days' : '30 Days'}
              </button>
            ))}
          </div>
          <Button variant="secondary" onClick={handleExportData} className="gap-2 text-[13px]">
            <Download className="w-4 h-4" /> Export CSV
          </Button>
          <Button onClick={() => setIsAlertDrawerOpen(true)} className="gap-2 text-[13px]">
            <Sliders className="w-4 h-4" /> Configure Alert Caps
          </Button>
        </div>
      </div>

      {/* KPIs Grid */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-5">
        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Accrued Spend</span>
            <p className="text-2xl font-bold text-gray-900">$38,542.00</p>
          </div>
          <div className="w-10 h-10 bg-gray-50 rounded-xl flex items-center justify-center border border-gray-100">
            <DollarSign className="w-5 h-5 text-gray-600" />
          </div>
        </Card>

        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Token Cost / 1M</span>
            <p className="text-2xl font-bold text-gray-900">$1.24</p>
          </div>
          <div className="w-10 h-10 bg-blue-50 rounded-xl flex items-center justify-center border border-blue-100">
            <TrendingUp className="w-5 h-5 text-blue-600" />
          </div>
        </Card>

        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Estimated Savings</span>
            <p className="text-2xl font-bold text-green-700">$14,820.00</p>
          </div>
          <div className="w-10 h-10 bg-green-50 rounded-xl flex items-center justify-center border border-green-100">
            <ArrowUpRight className="w-5 h-5 text-green-600" />
          </div>
        </Card>

        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Total Tokens</span>
            <p className="text-2xl font-bold text-gray-900">31.45B</p>
          </div>
          <div className="w-10 h-10 bg-purple-50 rounded-xl flex items-center justify-center border border-purple-100">
            <CreditCard className="w-5 h-5 text-purple-600" />
          </div>
        </Card>
      </div>

      {/* Charts section */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Cost breakdown Pie */}
        <Card className="p-6 flex flex-col justify-between">
          <div className="space-y-1 mb-4">
            <h3 className="text-base font-semibold text-gray-900 tracking-tight">Spend by Provider</h3>
            <p className="text-xs text-gray-400">Distribution of credit usage across AI endpoints</p>
          </div>
          <div className="h-56 flex items-center justify-center relative">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={providerBreakdown}
                  cx="50%"
                  cy="50%"
                  innerRadius={60}
                  outerRadius={80}
                  paddingAngle={4}
                  dataKey="value"
                >
                  {providerBreakdown.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip formatter={(val: number) => `$${val.toLocaleString()}`} />
              </PieChart>
            </ResponsiveContainer>
            <div className="absolute text-center">
              <span className="text-[11px] font-semibold text-gray-400 uppercase tracking-wider block">Total Spend</span>
              <span className="text-lg font-bold text-gray-900">$38.5K</span>
            </div>
          </div>
          <div className="space-y-2 mt-4 pt-4 border-t border-gray-100">
            {providerBreakdown.map((p) => (
              <div key={p.name} className="flex items-center justify-between text-xs font-semibold">
                <div className="flex items-center gap-2">
                  <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: p.color }}></span>
                  <span className="text-gray-600">{p.name}</span>
                </div>
                <div className="flex items-center gap-2 tabular-nums">
                  <span className="text-gray-900">${p.value.toLocaleString()}</span>
                  <span className="text-gray-400">({((p.value / 38542) * 100).toFixed(1)}%)</span>
                </div>
              </div>
            ))}
          </div>
        </Card>

        {/* Cost breakdown History */}
        <Card className="p-6 col-span-1 lg:col-span-2 flex flex-col justify-between">
          <div className="space-y-1 mb-4">
            <h3 className="text-base font-semibold text-gray-900 tracking-tight">Cumulative Cost Daily Run</h3>
            <p className="text-xs text-gray-400">Cost velocity tracking per provider during July</p>
          </div>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={dailyHistory}>
                <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#f3f4f6" />
                <XAxis dataKey="day" axisLine={false} tickLine={false} tick={{ fontSize: 11, fill: '#9ca3af' }} />
                <YAxis axisLine={false} tickLine={false} tick={{ fontSize: 11, fill: '#9ca3af' }} />
                <Tooltip formatter={(val: number) => `$${val}`} />
                <Legend iconType="circle" wrapperStyle={{ fontSize: 11, paddingTop: 10 }} />
                <Bar dataKey="Anthropic" stackId="a" fill="#8b5cf6" />
                <Bar dataKey="OpenAI" stackId="a" fill="#10b981" />
                <Bar dataKey="DeepSeek" stackId="a" fill="#3b82f6" />
                <Bar dataKey="Google" stackId="a" fill="#f97316" />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </Card>
      </div>

      {/* Tenancy break-down details */}
      <Card className="overflow-hidden">
        <div className="p-5 border-b border-gray-100 flex items-center justify-between">
          <div>
            <h3 className="text-base font-semibold text-gray-900 tracking-tight">Tenant Spend & Efficiency breakdown</h3>
            <p className="text-xs text-gray-400 mt-1">Tenant usage velocity compared to their current rate limits</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-sm text-left">
            <thead className="text-[13px] text-gray-500 bg-gray-50/30 border-b border-gray-200/60">
              <tr>
                <th className="px-6 py-4 font-medium">Tenant Client</th>
                <th className="px-6 py-4 font-medium text-right">Cumulative Spend</th>
                <th className="px-6 py-4 font-medium text-right">Estimated Tokens (Millions)</th>
                <th className="px-6 py-4 font-medium text-right">Allocated Rate Limits</th>
                <th className="px-6 py-4 font-medium text-right">Effective Cost / 1M Tokens</th>
                <th className="px-6 py-4 font-medium text-center">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {tenantBreakdown.map((tenant) => {
                const costPerMillion = (tenant.spend / (tenant.tokens)) * 1000;
                return (
                  <tr key={tenant.name} className="hover:bg-gray-50/50 transition-colors">
                    <td className="px-6 py-4.5 font-semibold text-gray-900 text-[13px]">{tenant.name}</td>
                    <td className="px-6 py-4.5 text-right font-bold text-gray-800 tabular-nums">${tenant.spend.toLocaleString()}</td>
                    <td className="px-6 py-4.5 text-right text-gray-500 tabular-nums">{tenant.tokens.toLocaleString()} M</td>
                    <td className="px-6 py-4.5 text-right font-medium text-gray-700 tabular-nums">{tenant.rps} RPS</td>
                    <td className="px-6 py-4.5 text-right text-gray-500 font-mono tabular-nums">${costPerMillion.toFixed(2)}</td>
                    <td className="px-6 py-4.5 text-center">
                      <Badge variant="success">Active billing</Badge>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </Card>

      {/* Spending limits controls drawer */}
      <Drawer 
        isOpen={isAlertDrawerOpen} 
        onClose={() => !isSavingAlerts && setIsAlertDrawerOpen(false)} 
        title="Configure Billing Safety Caps"
      >
        <form onSubmit={handleSaveAlerts} className="p-6 space-y-6 relative">
          {isSavingAlerts && (
            <div className="absolute inset-0 bg-white/70 backdrop-blur-[2px] z-10 flex flex-col items-center justify-center">
              <div className="w-8 h-8 border-2 border-gray-200 border-t-gray-900 rounded-full animate-spin mb-3"></div>
              <p className="text-xs font-semibold text-gray-900">Uploading billing limit protocols...</p>
            </div>
          )}

          <div className="bg-amber-50 border border-amber-200/80 p-4 rounded-xl text-amber-900 text-xs leading-normal flex gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-600 flex-shrink-0" />
            <div>
              <span className="font-bold">Caution on Hard Budget Thresholds</span>
              <p className="text-amber-700 mt-1">
                Crossing hard budget targets forces instant request throttling or failover loops across endpoints. Keep alert boundaries aligned with business needs.
              </p>
            </div>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <label className="text-xs font-semibold text-gray-600">Global Hard Spending Limit (Monthly)</label>
              <span className="text-xs font-bold text-gray-900 font-mono">${monthlyHardLimit.toLocaleString()}</span>
            </div>
            <input 
              type="range" 
              min="10000" 
              max="150000" 
              step="5000"
              value={monthlyHardLimit} 
              onChange={(e) => setMonthlyHardLimit(Number(e.target.value))}
              className="w-full h-1.5 bg-gray-100 rounded-full appearance-none cursor-pointer accent-gray-900"
            />
            <span className="text-[11px] text-gray-400 block">
              If the cumulative monthly spend crosses this limit, all secondary routing is blocked automatically until manual expansion.
            </span>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <label className="text-xs font-semibold text-gray-600">Warning Trigger Alert Threshold</label>
              <span className="text-xs font-bold text-gray-900 font-mono">{warningThreshold}% of hard limit</span>
            </div>
            <input 
              type="range" 
              min="50" 
              max="95" 
              step="5"
              value={warningThreshold} 
              onChange={(e) => setWarningThreshold(Number(e.target.value))}
              className="w-full h-1.5 bg-gray-100 rounded-full appearance-none cursor-pointer accent-gray-900"
            />
            <span className="text-[11px] text-gray-400 block">
              Triggers Slack/Webhook updates when monthly accrued costs exceed this proportion.
            </span>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-gray-600 block">Alert Webhook URL</label>
            <Input 
              required
              type="url" 
              value={alertWebhook} 
              onChange={(e) => setAlertWebhook(e.target.value)}
              placeholder="e.g. https://api.slack.com/services/..."
            />
            <span className="text-[11px] text-gray-400 block leading-normal">
              An HTTP POST payload containing budget state information will be dispatched immediately on breach.
            </span>
          </div>

          <div className="pt-4 border-t border-gray-100 flex items-center justify-end gap-3">
            <Button type="button" variant="secondary" onClick={() => setIsAlertDrawerOpen(false)} disabled={isSavingAlerts}>
              Cancel
            </Button>
            <Button type="submit" disabled={isSavingAlerts}>
              Commit Alert Limits
            </Button>
          </div>
        </form>
      </Drawer>
    </div>
  );
}
