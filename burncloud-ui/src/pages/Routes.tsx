import React, { useState } from 'react';
import { Button, Card, Badge, Drawer, Input } from '@/components/ui';
import { MOCK_ROUTES, MOCK_MODELS, Route } from '@/types';
import { MoreHorizontal, Plus, Upload, Play, GripVertical, Trash2 } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';

export function Routes() {
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [showToast, setShowToast] = useState(false);
  
  // Drawer state
  const [routeName, setRouteName] = useState('');
  const [primaryModel, setPrimaryModel] = useState('claude-fable-5');
  const [fallbackChain, setFallbackChain] = useState(['gpt-5.5', 'gemini-3.5-flash', 'DeepSeek-V4']);

  const handleCreateRoute = () => {
    setIsSaving(true);
    setTimeout(() => {
      setIsSaving(false);
      setIsDrawerOpen(false);
      setShowToast(true);
      setTimeout(() => setShowToast(false), 3000);
    }, 1500);
  };

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out relative">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Routes</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Define how requests move across models, providers, customers, budgets, and fallback rules.</p>
        </div>
        <div className="flex items-center gap-3">
          <Button variant="secondary" className="gap-2 text-[13px]"><Upload className="w-4 h-4" /> Import Config</Button>
          <Button variant="secondary" className="gap-2 text-[13px]"><Play className="w-4 h-4" /> Test Route</Button>
          <Button onClick={() => setIsDrawerOpen(true)} className="gap-2 text-[13px]"><Plus className="w-4 h-4" /> New Route</Button>
        </div>
      </div>

      <Card className="overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm text-left">
            <thead className="text-[13px] text-gray-500 bg-transparent border-b border-gray-200/60">
              <tr>
                <th className="px-6 py-4 font-medium">Route Name</th>
                <th className="px-6 py-4 font-medium">Environment</th>
                <th className="px-6 py-4 font-medium">Primary Model</th>
                <th className="px-6 py-4 font-medium">Fallback Chain</th>
                <th className="px-6 py-4 font-medium text-right">Traffic</th>
                <th className="px-6 py-4 font-medium text-right">Success</th>
                <th className="px-6 py-4 font-medium text-right">Latency</th>
                <th className="px-6 py-4 font-medium text-right">Cost/1M</th>
                <th className="px-6 py-4 font-medium text-center">Status</th>
                <th className="px-6 py-4 font-medium"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {MOCK_ROUTES.map((route) => (
                <tr key={route.id} className="hover:bg-gray-50/80 transition-colors group">
                  <td className="px-6 py-4 font-medium text-[13px] text-gray-900">{route.name}</td>
                  <td className="px-6 py-4">
                    <Badge variant={route.environment === 'Production' ? 'neutral' : 'warning'}>{route.environment}</Badge>
                  </td>
                  <td className="px-6 py-4 font-medium text-[13px] text-gray-900">{route.primaryModel}</td>
                  <td className="px-6 py-4 text-[13px] text-gray-500 max-w-[200px] truncate" title={route.fallbackChain.join(' → ')}>
                    {route.fallbackChain.join(' → ')}
                  </td>
                  <td className="px-6 py-4 text-right text-[13px] tabular-nums">{route.traffic}%</td>
                  <td className="px-6 py-4 text-right text-[13px] tabular-nums text-green-700 font-medium">{route.successRate}%</td>
                  <td className="px-6 py-4 text-right text-[13px] tabular-nums text-gray-500">{route.avgLatency}ms</td>
                  <td className="px-6 py-4 text-right text-[13px] tabular-nums text-gray-500">${route.costPer1M.toFixed(2)}</td>
                  <td className="px-6 py-4 text-center">
                    <Badge variant={route.status === 'Active' ? 'success' : 'neutral'}>{route.status}</Badge>
                  </td>
                  <td className="px-6 py-4 text-right">
                    <button className="text-gray-400 hover:text-gray-900 opacity-0 group-hover:opacity-100 transition-all p-1.5 rounded-lg hover:bg-gray-200/50">
                      <MoreHorizontal className="w-[18px] h-[18px]" />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      {/* Drawer */}
      <Drawer isOpen={isDrawerOpen} onClose={() => !isSaving && setIsDrawerOpen(false)} title="New Route">
        <div className="p-6 space-y-8 relative">
          
          {/* Progress Overlay when saving */}
          {isSaving && (
            <div className="absolute inset-0 bg-white/60 backdrop-blur-[1px] z-10 flex flex-col items-center justify-center">
              <div className="w-8 h-8 border-2 border-gray-200 border-t-gray-900 rounded-full animate-spin mb-4"></div>
              <p className="text-sm font-medium text-gray-900">Validating provider credentials...</p>
            </div>
          )}

          {/* Form Content */}
          <div className={cn("space-y-7", isSaving && "pointer-events-none opacity-50")}>
            <div className="space-y-2.5">
              <label className="text-[13px] font-medium text-gray-900">Route Name</label>
              <Input 
                placeholder="e.g. enterprise-chat-premium" 
                value={routeName} 
                onChange={e => setRouteName(e.target.value)} 
              />
            </div>

            <div className="space-y-2.5">
              <label className="text-[13px] font-medium text-gray-900">Environment</label>
              <select className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]">
                <option>Production</option>
                <option>Staging</option>
                <option>Development</option>
              </select>
            </div>

            <div className="space-y-2.5">
              <label className="text-[13px] font-medium text-gray-900">Primary Model</label>
              <div className="grid grid-cols-2 gap-3">
                {MOCK_MODELS.slice(0,4).map(model => (
                  <div 
                    key={model.id}
                    onClick={() => setPrimaryModel(model.name)}
                    className={cn(
                      "p-3.5 rounded-[14px] border cursor-pointer transition-all",
                      primaryModel === model.name 
                        ? "border-gray-900 bg-gray-900/5 shadow-sm ring-1 ring-inset ring-gray-900/10" 
                        : "border-gray-200/80 bg-white hover:border-gray-300 hover:bg-gray-50/50 shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
                    )}
                  >
                    <div className="text-[13px] font-medium text-gray-900">{model.name}</div>
                    <div className="text-[12px] text-gray-500 mt-0.5">{model.provider}</div>
                  </div>
                ))}
              </div>
            </div>

            <div className="space-y-2.5">
              <label className="text-[13px] font-medium text-gray-900 flex items-center justify-between">
                Fallback Chain
                <span className="text-[11px] text-gray-400 font-normal uppercase tracking-wider">Drag to reorder</span>
              </label>
              <div className="space-y-2">
                {fallbackChain.map((model, idx) => (
                  <div key={idx} className="flex items-center gap-3 p-3 bg-white border border-gray-200/80 rounded-[14px] hover:border-gray-300 shadow-[0_1px_2px_0_rgba(0,0,0,0.02)] transition-all group">
                    <GripVertical className="w-4 h-4 text-gray-300 cursor-grab active:cursor-grabbing" />
                    <span className="text-[13px] font-medium text-gray-700 flex-1">{model}</span>
                    <button className="text-gray-400 hover:text-red-600 opacity-0 group-hover:opacity-100 transition-opacity bg-red-50/0 hover:bg-red-50 p-1.5 rounded-lg">
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                ))}
              </div>
            </div>

            <div className="space-y-2.5">
              <label className="text-[13px] font-medium text-gray-900">Failure Conditions</label>
              <div className="space-y-3">
                {['Timeout > 8s', 'Error rate > 2%', 'Provider outage', 'Rate limit exceeded'].map((condition, idx) => (
                  <label key={idx} className="flex items-center gap-3 cursor-pointer group">
                    <input type="checkbox" defaultChecked={idx < 2} className="w-4 h-4 rounded-[4px] border-gray-300 text-gray-900 focus:ring-gray-900/20 transition-all cursor-pointer" />
                    <span className="text-[13px] text-gray-700 group-hover:text-gray-900 transition-colors">{condition}</span>
                  </label>
                ))}
              </div>
            </div>
            
            <div className="pt-8 pb-4 flex items-center justify-end gap-3">
              <Button variant="ghost" onClick={() => setIsDrawerOpen(false)}>Cancel</Button>
              <Button variant="secondary">Save as Draft</Button>
              <Button onClick={handleCreateRoute} disabled={isSaving}>
                {isSaving ? 'Creating...' : 'Create Route'}
              </Button>
            </div>
          </div>
        </div>
      </Drawer>

      {/* Toast Notification */}
      <AnimatePresence>
        {showToast && (
          <motion.div
            initial={{ opacity: 0, y: -20, x: 20 }}
            animate={{ opacity: 1, y: 0, x: 0 }}
            exit={{ opacity: 0, y: -20, scale: 0.95 }}
            className="fixed top-20 right-8 bg-white border border-gray-200 rounded-2xl shadow-lg p-4 z-50 flex items-start gap-4 min-w-[320px]"
          >
            <div className="w-8 h-8 rounded-full bg-green-50 flex items-center justify-center flex-shrink-0">
              <svg className="w-4 h-4 text-green-600" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" /></svg>
            </div>
            <div className="flex-1">
              <h4 className="text-sm font-medium text-gray-900">Route created</h4>
              <p className="text-sm text-gray-500 mt-0.5">"{routeName || 'enterprise-chat-premium'}" is now active in Staging.</p>
            </div>
            <Button variant="secondary" size="sm">View Route</Button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
