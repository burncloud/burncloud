import React, { useState } from 'react';
import { Button, Card, Badge, Drawer, Input } from '@/components/ui';
import { Users, DollarSign, Activity, AlertTriangle, Plus, Search, MoreHorizontal, ShieldCheck, Key, ArrowRight, TrendingUp } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';
import { MOCK_ROUTES } from '@/types';

interface Customer {
  id: string;
  name: string;
  environment: 'Production' | 'Staging' | 'Development';
  spend: number;
  budget: number;
  rpsLimit: number;
  defaultRoute: string;
  status: 'Active' | 'Suspended';
  keysCount: number;
  totalRequests: number;
}

const INITIAL_CUSTOMERS: Customer[] = [
  { id: 'c1', name: 'ETR Global', environment: 'Production', spend: 24820, budget: 25000, rpsLimit: 120, defaultRoute: 'production-chat-default', status: 'Active', keysCount: 4, totalRequests: 1204520 },
  { id: 'c2', name: 'NovaDesk', environment: 'Production', spend: 8420, budget: 15000, rpsLimit: 60, defaultRoute: 'cost-optimized-general', status: 'Active', keysCount: 2, totalRequests: 485120 },
  { id: 'c3', name: 'Internal', environment: 'Development', spend: 1280, budget: 5000, rpsLimit: 150, defaultRoute: 'coding-agent-premium', status: 'Active', keysCount: 8, totalRequests: 195200 },
  { id: 'c4', name: 'AeroTech', environment: 'Production', spend: 3120, budget: 10000, rpsLimit: 80, defaultRoute: 'production-chat-default', status: 'Active', keysCount: 3, totalRequests: 210850 },
  { id: 'c5', name: 'AlphaCorp', environment: 'Staging', spend: 1920, budget: 2000, rpsLimit: 30, defaultRoute: 'experimental-reasoning', status: 'Active', keysCount: 1, totalRequests: 94000 },
];

export function Customers() {
  const [customers, setCustomers] = useState<Customer[]>(INITIAL_CUSTOMERS);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCustomer, setSelectedCustomer] = useState<Customer | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isEditMode, setIsEditMode] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Form states
  const [formName, setFormName] = useState('');
  const [formEnv, setFormEnv] = useState<'Production' | 'Staging' | 'Development'>('Production');
  const [formBudget, setFormBudget] = useState(5000);
  const [formRps, setFormRps] = useState(50);
  const [formRoute, setFormRoute] = useState('production-chat-default');

  const filteredCustomers = customers.filter(c => 
    c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    c.defaultRoute.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleOpenNewDrawer = () => {
    setIsEditMode(false);
    setFormName('');
    setFormEnv('Production');
    setFormBudget(5000);
    setFormRps(50);
    setFormRoute('production-chat-default');
    setIsDrawerOpen(true);
  };

  const handleOpenEditDrawer = (cust: Customer) => {
    setSelectedCustomer(cust);
    setIsEditMode(true);
    setFormName(cust.name);
    setFormEnv(cust.environment);
    setFormBudget(cust.budget);
    setFormRps(cust.rpsLimit);
    setFormRoute(cust.defaultRoute);
    setIsDrawerOpen(true);
  };

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName) return;

    setIsSaving(true);
    setTimeout(() => {
      if (isEditMode && selectedCustomer) {
        setCustomers(prev => prev.map(c => c.id === selectedCustomer.id ? {
          ...c,
          name: formName,
          environment: formEnv,
          budget: Number(formBudget),
          rpsLimit: Number(formRps),
          defaultRoute: formRoute,
        } : c));
      } else {
        const newCust: Customer = {
          id: 'c_' + Date.now(),
          name: formName,
          environment: formEnv,
          spend: 0,
          budget: Number(formBudget),
          rpsLimit: Number(formRps),
          defaultRoute: formRoute,
          status: 'Active',
          keysCount: 1,
          totalRequests: 0
        };
        setCustomers(prev => [newCust, ...prev]);
      }
      setIsSaving(false);
      setIsDrawerOpen(false);
      setSelectedCustomer(null);
    }, 1000);
  };

  const handleToggleStatus = (id: string, currentStatus: 'Active' | 'Suspended') => {
    setCustomers(prev => prev.map(c => c.id === id ? {
      ...c,
      status: currentStatus === 'Active' ? 'Suspended' : 'Active'
    } : c));
  };

  const totalSpend = customers.reduce((sum, c) => sum + c.spend, 0);
  const totalBudget = customers.reduce((sum, c) => sum + c.budget, 0);
  const totalRequests = customers.reduce((sum, c) => sum + c.totalRequests, 0);
  const criticalCount = customers.filter(c => (c.spend / c.budget) >= 0.9).length;

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Customers</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Manage tenant metadata, dynamic rate limits, model access budgets, and route policy mapping.</p>
        </div>
        <Button onClick={handleOpenNewDrawer} className="gap-2 text-[13px] self-start sm:self-auto">
          <Plus className="w-4 h-4" /> Add Tenant Customer
        </Button>
      </div>

      {/* Metrics Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-5">
        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Total Tenants</span>
            <p className="text-2xl font-bold text-gray-900">{customers.length}</p>
          </div>
          <div className="w-10 h-10 bg-gray-50 rounded-xl flex items-center justify-center border border-gray-100">
            <Users className="w-5 h-5 text-gray-600" />
          </div>
        </Card>

        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Active Month Spend</span>
            <p className="text-2xl font-bold text-gray-900">${(totalSpend / 1000).toFixed(2)}K</p>
          </div>
          <div className="w-10 h-10 bg-green-50 rounded-xl flex items-center justify-center border border-green-100">
            <DollarSign className="w-5 h-5 text-green-600" />
          </div>
        </Card>

        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Total Demands</span>
            <p className="text-2xl font-bold text-gray-900">{(totalRequests / 1000000).toFixed(2)}M Req</p>
          </div>
          <div className="w-10 h-10 bg-blue-50 rounded-xl flex items-center justify-center border border-blue-100">
            <Activity className="w-5 h-5 text-blue-600" />
          </div>
        </Card>

        <Card className={cn("p-5 flex items-center justify-between transition-colors border", criticalCount > 0 ? "border-amber-200 bg-amber-50/10" : "border-gray-200/60")}>
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Budget Alerts</span>
            <p className={cn("text-2xl font-bold", criticalCount > 0 ? "text-amber-700" : "text-gray-900")}>{criticalCount} Critical</p>
          </div>
          <div className={cn("w-10 h-10 rounded-xl flex items-center justify-center border", criticalCount > 0 ? "bg-amber-100 border-amber-200" : "bg-gray-50 border-gray-100")}>
            <AlertTriangle className={cn("w-5 h-5", criticalCount > 0 ? "text-amber-600" : "text-gray-600")} />
          </div>
        </Card>
      </div>

      {/* Customer List Card */}
      <Card className="overflow-hidden">
        <div className="p-5 border-b border-gray-100 flex items-center gap-4">
          <div className="relative flex-1 max-w-md">
            <Search className="w-[15px] h-[15px] absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <input 
              type="text" 
              placeholder="Search tenants or active policies..." 
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full h-9 bg-gray-50/80 border border-gray-200/80 rounded-[10px] pl-9 pr-4 text-[13px] focus:bg-white focus:ring-1 focus:ring-gray-300 focus:border-gray-300 focus:outline-none transition-all placeholder:text-gray-400"
            />
          </div>
          <div className="flex-1"></div>
          <span className="text-xs text-gray-400 font-medium">Showing {filteredCustomers.length} of {customers.length} tenants</span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-sm text-left">
            <thead className="text-[13px] text-gray-500 bg-gray-50/30 border-b border-gray-200/60">
              <tr>
                <th className="px-6 py-3.5 font-medium">Tenant Name</th>
                <th className="px-6 py-3.5 font-medium">Environment</th>
                <th className="px-6 py-3.5 font-medium">Monthly Cost Track</th>
                <th className="px-6 py-3.5 font-medium text-right">Quota Rate Limit</th>
                <th className="px-6 py-3.5 font-medium">Assigned Default Route</th>
                <th className="px-6 py-3.5 font-medium text-center">API Keys</th>
                <th className="px-6 py-3.5 font-medium text-center">Status</th>
                <th className="px-6 py-3.5 font-medium"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {filteredCustomers.map((cust) => {
                const ratio = cust.spend / cust.budget;
                const isCritical = ratio >= 0.9;
                const isWarning = ratio >= 0.6 && ratio < 0.9;
                
                return (
                  <tr key={cust.id} className="hover:bg-gray-50/80 transition-colors group cursor-pointer" onClick={() => handleOpenEditDrawer(cust)}>
                    <td className="px-6 py-4.5 font-medium text-[13px] text-gray-900">
                      <div className="flex items-center gap-2">
                        {cust.name}
                        {isCritical && <span className="w-1.5 h-1.5 rounded-full bg-red-500 animate-pulse"></span>}
                      </div>
                    </td>
                    <td className="px-6 py-4.5">
                      <Badge variant={cust.environment === 'Production' ? 'brand' : cust.environment === 'Staging' ? 'warning' : 'neutral'}>
                        {cust.environment}
                      </Badge>
                    </td>
                    <td className="px-6 py-4.5">
                      <div className="space-y-1.5 max-w-[160px]">
                        <div className="flex justify-between text-xs font-semibold tabular-nums">
                          <span className={cn(isCritical ? "text-red-600 font-bold" : "text-gray-900")}>${cust.spend.toLocaleString()}</span>
                          <span className="text-gray-400">/ ${cust.budget.toLocaleString()}</span>
                        </div>
                        <div className="w-full h-1.5 bg-gray-100 rounded-full overflow-hidden">
                          <div 
                            className={cn("h-full rounded-full transition-all duration-500", 
                              isCritical ? "bg-red-500" : isWarning ? "bg-amber-400" : "bg-emerald-500"
                            )} 
                            style={{ width: `${Math.min(ratio * 100, 100)}%` }}
                          />
                        </div>
                      </div>
                    </td>
                    <td className="px-6 py-4.5 text-right font-medium text-gray-700 tabular-nums text-[13px]">
                      {cust.rpsLimit} RPS
                    </td>
                    <td className="px-6 py-4.5 text-[13px] text-gray-500 font-mono">
                      {cust.defaultRoute}
                    </td>
                    <td className="px-6 py-4.5 text-center font-medium text-gray-600">
                      <div className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-gray-50 border border-gray-100 text-[11px] font-mono">
                        <Key className="w-3 h-3 text-gray-400" /> {cust.keysCount}
                      </div>
                    </td>
                    <td className="px-6 py-4.5 text-center" onClick={(e) => e.stopPropagation()}>
                      <button 
                        onClick={() => handleToggleStatus(cust.id, cust.status)}
                        className="focus:outline-none"
                      >
                        <Badge variant={cust.status === 'Active' ? 'success' : 'neutral'}>
                          {cust.status}
                        </Badge>
                      </button>
                    </td>
                    <td className="px-6 py-4.5 text-right">
                      <Button variant="ghost" size="sm" className="opacity-0 group-hover:opacity-100 transition-all">
                        Edit
                      </Button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </Card>

      {/* Drawer Form / Edit Details */}
      <Drawer 
        isOpen={isDrawerOpen} 
        onClose={() => !isSaving && setIsDrawerOpen(false)} 
        title={isEditMode ? `Tenant Profile: ${selectedCustomer?.name}` : "Create Tenant Customer"}
      >
        <form onSubmit={handleSave} className="p-6 space-y-6 relative">
          {isSaving && (
            <div className="absolute inset-0 bg-white/70 backdrop-blur-[2px] z-10 flex flex-col items-center justify-center">
              <div className="w-8 h-8 border-2 border-gray-200 border-t-gray-900 rounded-full animate-spin mb-3"></div>
              <p className="text-xs font-semibold text-gray-900">Applying tenancy controls...</p>
            </div>
          )}

          {isEditMode && selectedCustomer && (
            <div className="bg-gray-50 border border-gray-200/60 p-4 rounded-xl space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-gray-400 uppercase tracking-wider">Historical Demands</span>
                <span className="text-xs text-gray-500 font-medium font-mono">ID: {selectedCustomer.id}</span>
              </div>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-gray-400 block text-xs">Total API Demands</span>
                  <span className="font-semibold text-gray-800 font-mono text-[13px]">{selectedCustomer.totalRequests.toLocaleString()} requests</span>
                </div>
                <div>
                  <span className="text-gray-400 block text-xs">Spend Velocity</span>
                  <span className="font-semibold text-green-700 font-mono text-[13px]">
                    ${(selectedCustomer.spend / 30).toFixed(2)} / day
                  </span>
                </div>
              </div>
            </div>
          )}

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-gray-600 block">Tenant Customer Name</label>
            <Input 
              required
              type="text" 
              placeholder="e.g. AeroTech Corp" 
              value={formName}
              onChange={(e) => setFormName(e.target.value)}
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-600 block">Environment</label>
              <select 
                value={formEnv}
                onChange={(e) => setFormEnv(e.target.value as any)}
                className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
              >
                <option value="Production">Production</option>
                <option value="Staging">Staging</option>
                <option value="Development">Development</option>
              </select>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-600 block">Default Route Policy</label>
              <select 
                value={formRoute}
                onChange={(e) => setFormRoute(e.target.value)}
                className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
              >
                {MOCK_ROUTES.map(r => (
                  <option key={r.id} value={r.name}>{r.name}</option>
                ))}
              </select>
            </div>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <label className="text-xs font-semibold text-gray-600">Rate Limit Quota</label>
              <span className="text-xs font-bold text-gray-900 font-mono">{formRps} RPS</span>
            </div>
            <input 
              type="range" 
              min="5" 
              max="500" 
              step="5"
              value={formRps} 
              onChange={(e) => setFormRps(Number(e.target.value))}
              className="w-full h-1.5 bg-gray-100 rounded-full appearance-none cursor-pointer accent-gray-900"
            />
            <span className="text-[11px] text-gray-400 leading-normal block">
              Throttle traffic automatically when this client exceeds the specified requests per second rate.
            </span>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <label className="text-xs font-semibold text-gray-600">Monthly Budget Threshold</label>
              <span className="text-xs font-bold text-gray-900 font-mono">${formBudget.toLocaleString()}</span>
            </div>
            <input 
              type="range" 
              min="500" 
              max="50000" 
              step="500"
              value={formBudget} 
              onChange={(e) => setFormBudget(Number(e.target.value))}
              className="w-full h-1.5 bg-gray-100 rounded-full appearance-none cursor-pointer accent-gray-900"
            />
            <span className="text-[11px] text-gray-400 leading-normal block">
              Automated notifications or model failovers will trigger once cumulative tenant spend crosses this threshold.
            </span>
          </div>

          {isEditMode && selectedCustomer && (
            <div className="pt-2 border-t border-gray-100 space-y-3">
              <label className="text-xs font-semibold text-gray-600 block">Client Health Status Actions</label>
              <div className="flex items-center gap-3">
                <Button 
                  type="button" 
                  variant="secondary" 
                  className="flex-1 text-xs gap-1.5"
                  onClick={() => handleToggleStatus(selectedCustomer.id, selectedCustomer.status)}
                >
                  {selectedCustomer.status === 'Active' ? 'Suspend Tenant Access' : 'Reactivate Tenant Access'}
                </Button>
              </div>
            </div>
          )}

          <div className="pt-4 border-t border-gray-100 flex items-center justify-end gap-3">
            <Button type="button" variant="secondary" onClick={() => setIsDrawerOpen(false)} disabled={isSaving}>
              Cancel
            </Button>
            <Button type="submit" disabled={isSaving}>
              {isEditMode ? 'Update Controls' : 'Create Tenant'}
            </Button>
          </div>
        </form>
      </Drawer>
    </div>
  );
}
