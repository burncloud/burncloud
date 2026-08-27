import React, { useState } from 'react';
import { Button, Card, Badge, Drawer, Input } from '@/components/ui';
import { Plus, Download, KeyRound, Copy, Check, Shield, Server, Trash2 } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';

interface ApiKeyItem {
  name: string;
  key: string;
  owner: string;
  customer: string;
  perms: string;
  lastUsed: string;
  usage: string;
  rateLimit: string;
  status: 'Active' | 'Limited' | 'Revoked';
}

const INITIAL_KEYS: ApiKeyItem[] = [
  { name: 'burncloud-prod-key', key: 'demo-bk-production-credential', owner: 'Wei Huang', customer: 'Internal', perms: 'Full Access', lastUsed: '2 mins ago', usage: '4.8M', rateLimit: '5,000 RPM', status: 'Active' },
  { name: 'etr-global-chat', key: 'demo-bk-customer-credential', owner: 'ETR Global', customer: 'ETR Global', perms: 'Chat Routes Only', lastUsed: '8 secs ago', usage: '1.2M', rateLimit: '1,000 RPM', status: 'Active' },
  { name: 'demo-customer-staging', key: 'demo-bk-staging-credential', owner: 'Demo Team', customer: 'Demo', perms: 'Staging Only', lastUsed: '2 days ago', usage: '84K', rateLimit: '100 RPM', status: 'Limited' },
];

export function APIKeys() {
  const [keys, setKeys] = useState<ApiKeyItem[]>(INITIAL_KEYS);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  // Form states
  const [formName, setFormName] = useState('');
  const [formCustomer, setFormCustomer] = useState('Internal');
  const [formPerms, setFormPerms] = useState('Full Access');
  const [formRateLimit, setFormRateLimit] = useState('1,000 RPM');

  // Success state
  const [newlyCreatedKey, setNewlyCreatedKey] = useState<string | null>(null);

  const handleCreateKey = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName) return;

    setIsSaving(true);

    // Simulate key generation and network delay
    setTimeout(() => {
      const entropy = Math.random().toString(16).substring(2, 12) + Math.random().toString(16).substring(2, 12);
      const bkKey = `demo-bk-${entropy}`;

      const newKey: ApiKeyItem = {
        name: formName,
        key: bkKey,
        owner: 'Wei Huang',
        customer: formCustomer,
        perms: formPerms,
        lastUsed: 'Never',
        usage: '0',
        rateLimit: formRateLimit,
        status: 'Active',
      };

      setKeys((prev) => [newKey, ...prev]);
      setNewlyCreatedKey(bkKey);
      setIsSaving(false);
    }, 1200);
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(text);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleRevokeKey = (name: string) => {
    setKeys((prev) =>
      prev.map((k) => (k.name === name ? { ...k, status: 'Revoked' as const } : k))
    );
  };

  const handleOpenDrawer = () => {
    setFormName('');
    setFormCustomer('Internal');
    setFormPerms('Full Access');
    setFormRateLimit('1,000 RPM');
    setNewlyCreatedKey(null);
    setIsDrawerOpen(true);
  };

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">API Keys</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Manage access credentials and rate limits for customers and internal teams.</p>
        </div>
        <div className="flex items-center gap-3">
          <Button variant="secondary" className="gap-2 text-[13px]"><Download className="w-4 h-4" /> Export Usage</Button>
          <Button onClick={handleOpenDrawer} className="gap-2 text-[13px]"><Plus className="w-4 h-4" /> Create Key</Button>
        </div>
      </div>

      {/* Main Card Table */}
      <Card className="overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm text-left">
            <thead className="text-[13px] text-gray-500 bg-transparent border-b border-gray-200/60">
              <tr>
                <th className="px-6 py-4 font-medium">Key Name</th>
                <th className="px-6 py-4 font-medium">Customer</th>
                <th className="px-6 py-4 font-medium">Permissions</th>
                <th className="px-6 py-4 font-medium">Last Used</th>
                <th className="px-6 py-4 font-medium text-right">Usage</th>
                <th className="px-6 py-4 font-medium text-right">Rate Limit</th>
                <th className="px-6 py-4 font-medium text-center">Status</th>
                <th className="px-6 py-4"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {keys.map((keyItem) => (
                <tr key={keyItem.name} className={cn(
                  "hover:bg-gray-50/80 transition-colors group",
                  keyItem.status === 'Revoked' && "opacity-55 hover:bg-transparent"
                )}>
                  <td className="px-6 py-4">
                    <div className="flex items-center gap-2.5">
                      <KeyRound className={cn(
                        "w-[15px] h-[15px]",
                        keyItem.status === 'Active' ? "text-gray-400 group-hover:text-gray-700 transition-colors" : "text-gray-300"
                      )} />
                      <div>
                        <span className="font-semibold text-[13.5px] text-gray-900 block">{keyItem.name}</span>
                        <span className="font-mono text-[10.5px] text-gray-400 mt-0.5 block select-all">
                          {keyItem.key.substring(0, 10)}••••••••••••{keyItem.key.substring(keyItem.key.length - 4)}
                        </span>
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4 text-[13px] text-gray-900">{keyItem.customer}</td>
                  <td className="px-6 py-4 text-[13px] text-gray-500">{keyItem.perms}</td>
                  <td className="px-6 py-4 text-[13px] text-gray-500">{keyItem.lastUsed}</td>
                  <td className="px-6 py-4 text-right text-[13px] tabular-nums text-gray-900 font-medium">{keyItem.usage}</td>
                  <td className="px-6 py-4 text-right text-[13px] tabular-nums text-gray-500">{keyItem.rateLimit}</td>
                  <td className="px-6 py-4 text-center">
                    <Badge variant={
                      keyItem.status === 'Active' ? 'success' :
                      keyItem.status === 'Limited' ? 'warning' : 'neutral'
                    }>
                      {keyItem.status}
                    </Badge>
                  </td>
                  <td className="px-6 py-4 text-right">
                    {keyItem.status !== 'Revoked' ? (
                      <div className="flex items-center justify-end gap-2">
                        <button
                          onClick={() => handleCopy(keyItem.key)}
                          className="text-gray-400 hover:text-gray-900 p-1.5 rounded-lg hover:bg-gray-100 transition-all opacity-0 group-hover:opacity-100 focus:opacity-100"
                          title="Copy Full Key"
                        >
                          {copiedKey === keyItem.key ? <Check className="w-4 h-4 text-green-600 animate-scale" /> : <Copy className="w-4 h-4" />}
                        </button>
                        <button
                          onClick={() => handleRevokeKey(keyItem.name)}
                          className="text-gray-400 hover:text-red-600 p-1.5 rounded-lg hover:bg-red-50 transition-all opacity-0 group-hover:opacity-100 focus:opacity-100"
                          title="Revoke Key"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </div>
                    ) : (
                      <span className="text-[11px] font-semibold uppercase tracking-wider text-gray-400 font-mono">Inactive</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      {/* Slide-over Drawer for API Key Creation */}
      <Drawer
        isOpen={isDrawerOpen}
        onClose={() => !isSaving && setIsDrawerOpen(false)}
        title="Create API Credential"
      >
        <div className="p-6 space-y-7 relative">
          {isSaving && (
            <div className="absolute inset-0 bg-white/70 backdrop-blur-[1.5px] z-10 flex flex-col items-center justify-center">
              <div className="w-8 h-8 border-2 border-gray-200 border-t-gray-900 rounded-full animate-spin mb-3"></div>
              <p className="text-xs font-semibold text-gray-900">Provisioning secure hashing rules...</p>
            </div>
          )}

          <AnimatePresence mode="wait">
            {!newlyCreatedKey ? (
              <motion.form
                key="form"
                onSubmit={handleCreateKey}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -10 }}
                className="space-y-6"
              >
                <div className="space-y-1.5">
                  <label className="text-xs font-semibold text-gray-600 block">Credential Name</label>
                  <Input
                    required
                    type="text"
                    placeholder="e.g. staging-customer-analytics"
                    value={formName}
                    onChange={(e) => setFormName(e.target.value)}
                  />
                  <span className="text-[11px] text-gray-400 block leading-normal">
                    Give this key a unique descriptive name to audit inside the analytics gateway.
                  </span>
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-1.5">
                    <label className="text-xs font-semibold text-gray-600 block">Customer</label>
                    <Input
                      type="text"
                      placeholder="Internal, or company name"
                      value={formCustomer}
                      onChange={(e) => setFormCustomer(e.target.value)}
                    />
                  </div>

                  <div className="space-y-1.5">
                    <label className="text-xs font-semibold text-gray-600 block">Rate Limit</label>
                    <select
                      value={formRateLimit}
                      onChange={(e) => setFormRateLimit(e.target.value)}
                      className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
                    >
                      <option value="100 RPM">100 RPM (Hobbyist)</option>
                      <option value="1,000 RPM">1,000 RPM (Standard)</option>
                      <option value="5,000 RPM">5,000 RPM (Enterprise)</option>
                      <option value="10,000 RPM">10,000 RPM (Unlimited)</option>
                    </select>
                  </div>
                </div>

                <div className="space-y-1.5">
                  <label className="text-xs font-semibold text-gray-600 block">Permissions</label>
                  <select
                    value={formPerms}
                    onChange={(e) => setFormPerms(e.target.value)}
                    className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
                  >
                    <option value="Full Access">Full Access (All models, routers, and configurations)</option>
                    <option value="Chat Routes Only">Chat Routes Only (Only routes under the /chat path)</option>
                    <option value="Staging Only">Staging Only (Cannot trigger Production models)</option>
                    <option value="ReadOnly">Auditor Read-Only</option>
                  </select>
                </div>

                <div className="pt-6 border-t border-gray-100 flex items-center justify-end gap-3">
                  <Button type="button" variant="secondary" onClick={() => setIsDrawerOpen(false)}>
                    Cancel
                  </Button>
                  <Button type="submit">
                    Generate Access Key
                  </Button>
                </div>
              </motion.form>
            ) : (
              <motion.div
                key="success"
                initial={{ opacity: 0, scale: 0.98 }}
                animate={{ opacity: 1, scale: 1 }}
                className="space-y-6 text-center py-4"
              >
                <div className="w-12 h-12 bg-green-50 rounded-full flex items-center justify-center border border-green-100 mx-auto mb-2">
                  <Check className="w-6 h-6 text-green-600" />
                </div>
                <div>
                  <h3 className="text-lg font-semibold text-gray-900">Key Generated Successfully</h3>
                  <p className="text-xs text-gray-500 mt-1.5 max-w-sm mx-auto leading-relaxed">
                    Make sure to copy this credential now. For security purposes, you won't be able to view it again.
                  </p>
                </div>

                {/* High Tech Glass Card showing key */}
                <div className="bg-gray-950 p-4.5 rounded-2xl text-left border border-gray-900 shadow-xl relative overflow-hidden">
                  <div className="absolute top-0 right-0 w-32 h-32 bg-green-600/5 rounded-full blur-2xl pointer-events-none" />
                  <span className="text-[10px] font-bold text-gray-500 uppercase tracking-widest block mb-2 font-mono">YOUR PRIVATE TOKEN</span>
                  <div className="flex items-center justify-between gap-3 bg-gray-900/50 p-3 rounded-xl border border-gray-900">
                    <span className="font-mono text-xs text-green-400 break-all select-all font-semibold">
                      {newlyCreatedKey}
                    </span>
                    <button
                      type="button"
                      onClick={() => handleCopy(newlyCreatedKey)}
                      className="bg-gray-800 hover:bg-gray-700 text-white p-2 rounded-lg border border-gray-700 transition-all active:scale-95 flex-shrink-0 cursor-pointer"
                    >
                      {copiedKey === newlyCreatedKey ? <Check className="w-4 h-4 text-green-400" /> : <Copy className="w-4 h-4 text-gray-300" />}
                    </button>
                  </div>
                </div>

                <div className="pt-4 flex justify-center">
                  <Button type="button" variant="primary" onClick={() => setIsDrawerOpen(false)} className="w-full">
                    Done, return to panel
                  </Button>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </Drawer>
    </div>
  );
}

