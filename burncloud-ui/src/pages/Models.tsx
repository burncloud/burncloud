import React, { useState } from 'react';
import { Button, Card, Badge, Input, Drawer } from '@/components/ui';
import { MOCK_MODELS, Model } from '@/types';
import { Search, Filter, Cpu, Check, TrendingDown, DollarSign, Hourglass, Percent } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';

export function Models() {
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedProvider, setSelectedProvider] = useState<string>('All');
  const [selectedQuality, setSelectedQuality] = useState<string>('All');
  const [selectedModel, setSelectedModel] = useState<Model | null>(null);

  // Derive unique providers for filter dropdown
  const providersList = ['All', ...Array.from(new Set(MOCK_MODELS.map((m) => m.provider)))];

  // Filtering logic
  const filteredModels = MOCK_MODELS.filter((model) => {
    const matchesSearch =
      model.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      model.provider.toLowerCase().includes(searchTerm.toLowerCase()) ||
      model.tags.some((t) => t.toLowerCase().includes(searchTerm.toLowerCase()));

    const matchesProvider = selectedProvider === 'All' || model.provider === selectedProvider;

    const matchesQuality =
      selectedQuality === 'All' ||
      (selectedQuality === 'Elite (>95)' && model.quality >= 95) ||
      (selectedQuality === 'Standard (<95)' && model.quality < 95);

    return matchesSearch && matchesProvider && matchesQuality;
  });

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
      {/* Header with Search and Filter bar */}
      <div className="flex flex-col xl:flex-row xl:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Models</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Central catalog of all available AI models across connected providers.</p>
        </div>
        
        <div className="flex flex-wrap items-center gap-3">
          {/* Search Box */}
          <div className="relative w-full sm:w-64">
            <Search className="w-[15px] h-[15px] absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <Input 
              placeholder="Search by name, tags..." 
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-9 bg-white text-[13px] h-10 w-full" 
            />
          </div>

          {/* Provider Filter */}
          <div className="flex items-center gap-1.5 bg-white border border-gray-200/80 rounded-xl px-3 h-10 shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]">
            <span className="text-[11px] font-semibold text-gray-400 uppercase tracking-wider mr-1">Provider:</span>
            <select
              value={selectedProvider}
              onChange={(e) => setSelectedProvider(e.target.value)}
              className="text-[13px] bg-transparent font-medium text-gray-700 outline-none cursor-pointer"
            >
              {providersList.map((prov) => (
                <option key={prov} value={prov}>{prov}</option>
              ))}
            </select>
          </div>

          {/* Quality Filter */}
          <div className="flex items-center gap-1.5 bg-white border border-gray-200/80 rounded-xl px-3 h-10 shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]">
            <span className="text-[11px] font-semibold text-gray-400 uppercase tracking-wider mr-1">Quality:</span>
            <select
              value={selectedQuality}
              onChange={(e) => setSelectedQuality(e.target.value)}
              className="text-[13px] bg-transparent font-medium text-gray-700 outline-none cursor-pointer"
            >
              <option value="All">All Quality tiers</option>
              <option value="Elite (>95)">Elite tier (&gt;95)</option>
              <option value="Standard (<95)">Standard tier (&lt;95)</option>
            </select>
          </div>

          {/* Reset button */}
          {(searchTerm || selectedProvider !== 'All' || selectedQuality !== 'All') && (
            <Button 
              variant="ghost" 
              size="sm"
              onClick={() => {
                setSearchTerm('');
                setSelectedProvider('All');
                setSelectedQuality('All');
              }}
              className="text-[12px] text-gray-500 hover:text-gray-900"
            >
              Clear filters
            </Button>
          )}
        </div>
      </div>

      {/* Grid of cards */}
      {filteredModels.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
          {filteredModels.map((model, i) => (
            <motion.div
              key={model.id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.04 }}
              layout
              onClick={() => setSelectedModel(model)}
            >
              <Card className="flex flex-col h-full hover:shadow-[0_12px_40px_-6px_rgba(0,0,0,0.04)] transition-all duration-300 group cursor-pointer border-gray-200/60 hover:border-gray-300/90 hover:-translate-y-0.5">
                <div className="p-6 border-b border-gray-100/80 flex-1">
                  <div className="flex items-start justify-between mb-4">
                    <div className="flex items-center gap-3.5">
                      <div className="w-11 h-11 rounded-2xl bg-gradient-to-br from-gray-50 to-gray-100 flex items-center justify-center border border-gray-200/60 shadow-[inset_0_1px_2px_rgba(255,255,255,1)] group-hover:bg-gradient-to-br group-hover:from-gray-900 group-hover:to-gray-850 group-hover:border-transparent transition-all duration-300">
                        <Cpu className="w-5 h-5 text-gray-500 group-hover:text-white transition-colors duration-300" />
                      </div>
                      <div>
                        <h3 className="font-semibold text-[15px] text-gray-900 tracking-tight group-hover:text-orange-600 transition-colors">{model.name}</h3>
                        <p className="text-[12.5px] text-gray-400 mt-0.5">{model.provider}</p>
                      </div>
                    </div>
                    <Badge variant="brand" className="bg-blue-50/50 text-blue-600 border-blue-100 font-medium">{model.quality} index</Badge>
                  </div>
                  
                  <div className="flex flex-wrap gap-1.5 mb-6">
                    {model.tags.map(tag => (
                      <Badge key={tag} variant="neutral" className="bg-gray-50/50 text-gray-500 font-normal">{tag}</Badge>
                    ))}
                  </div>

                  <div className="grid grid-cols-2 gap-y-4 gap-x-3 text-[13px]">
                    <div>
                      <span className="text-gray-400 text-[11px] block mb-1">Context</span>
                      <span className="font-semibold text-gray-800 tabular-nums">{model.contextWindow}</span>
                    </div>
                    <div>
                      <span className="text-gray-400 text-[11px] block mb-1">Latency</span>
                      <span className="font-semibold text-gray-850 tabular-nums">{model.latency}ms</span>
                    </div>
                    <div>
                      <span className="text-gray-400 text-[11px] block mb-1">Input /1M</span>
                      <span className="font-semibold text-gray-800 tabular-nums">${model.inputCost.toFixed(2)}</span>
                    </div>
                    <div>
                      <span className="text-gray-400 text-[11px] block mb-1">Output /1M</span>
                      <span className="font-semibold text-gray-800 tabular-nums">${model.outputCost.toFixed(2)}</span>
                    </div>
                  </div>
                </div>
                
                <div className="px-6 py-3.5 bg-gray-50/50 rounded-b-[20px] flex items-center justify-between text-[13px] border-t border-gray-100/50">
                  <span className="text-gray-500 text-xs font-medium">Reliability score</span>
                  <div className="flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse"></span>
                    <span className="font-bold text-green-700 font-mono text-xs">{model.reliability}%</span>
                  </div>
                </div>
              </Card>
            </motion.div>
          ))}
        </div>
      ) : (
        <Card className="p-16 text-center border-dashed border-2 border-gray-200">
          <Cpu className="w-10 h-10 text-gray-300 mx-auto mb-4" />
          <h3 className="text-sm font-semibold text-gray-900">No models match your search</h3>
          <p className="text-xs text-gray-500 mt-1 max-w-sm mx-auto">Try adjusting your keyword filter or changing your provider dropdown menu selection.</p>
          <Button 
            variant="secondary" 
            size="sm"
            onClick={() => {
              setSearchTerm('');
              setSelectedProvider('All');
              setSelectedQuality('All');
            }}
            className="mt-4 text-[12px]"
          >
            Clear Search Filter
          </Button>
        </Card>
      )}

      {/* Slide-over Drawer for Model Deep-dive Analytics */}
      <Drawer
        isOpen={!!selectedModel}
        onClose={() => setSelectedModel(null)}
        title={selectedModel ? `${selectedModel.name} Audit` : 'Model Detail'}
      >
        {selectedModel && (
          <div className="p-6 space-y-8">
            {/* Meta header card */}
            <div className="flex items-center gap-4 p-4.5 bg-gray-50 rounded-2xl border border-gray-100">
              <div className="w-12 h-12 rounded-xl bg-gray-900 flex items-center justify-center text-white">
                <Cpu className="w-6 h-6" />
              </div>
              <div>
                <h3 className="text-[16px] font-bold text-gray-900 tracking-tight">{selectedModel.name}</h3>
                <p className="text-xs text-gray-500 mt-0.5">Offered by {selectedModel.provider}</p>
              </div>
            </div>

            {/* Performance Metric Blocks */}
            <div className="grid grid-cols-2 gap-4">
              <div className="p-4 bg-white rounded-xl border border-gray-100 shadow-sm">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-[11px] font-bold text-gray-400 uppercase tracking-wider">Quality Score</span>
                  <Percent className="w-4 h-4 text-blue-500" />
                </div>
                <div className="text-2xl font-bold text-gray-900">{selectedModel.quality}%</div>
                <p className="text-[10px] text-gray-400 mt-1">Relative to GPT-4o base benchmarks.</p>
              </div>

              <div className="p-4 bg-white rounded-xl border border-gray-100 shadow-sm">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-[11px] font-bold text-gray-400 uppercase tracking-wider">Reliability</span>
                  <Check className="w-4 h-4 text-green-500" />
                </div>
                <div className="text-2xl font-bold text-green-600">{selectedModel.reliability}%</div>
                <p className="text-[10px] text-gray-400 mt-1">SLA success rate on current node.</p>
              </div>
            </div>

            {/* In-depth Specs List */}
            <div className="space-y-4">
              <h4 className="text-xs font-bold text-gray-400 uppercase tracking-wider">Detailed Specifications</h4>
              <div className="divide-y divide-gray-100 border border-gray-100 rounded-2xl overflow-hidden bg-white">
                <div className="flex items-center justify-between p-3.5 text-[13px]">
                  <span className="text-gray-500">Context Window</span>
                  <span className="font-semibold text-gray-900">{selectedModel.contextWindow}</span>
                </div>
                <div className="flex items-center justify-between p-3.5 text-[13px]">
                  <span className="text-gray-500">Average Node Latency</span>
                  <span className="font-semibold text-gray-900">{selectedModel.latency} ms</span>
                </div>
                <div className="flex items-center justify-between p-3.5 text-[13px]">
                  <span className="text-gray-500">Input Token Pricing</span>
                  <span className="font-semibold text-gray-900 text-blue-600">${selectedModel.inputCost.toFixed(2)} per 1M</span>
                </div>
                <div className="flex items-center justify-between p-3.5 text-[13px]">
                  <span className="text-gray-500">Output Token Pricing</span>
                  <span className="font-semibold text-gray-900 text-blue-600">${selectedModel.outputCost.toFixed(2)} per 1M</span>
                </div>
              </div>
            </div>

            {/* Steve Jobs Design philosophy quotation/metric callout */}
            <div className="p-5.5 bg-gradient-to-br from-gray-950 to-gray-900 text-white rounded-[20px] relative overflow-hidden shadow-xl border border-gray-900">
              <div className="absolute top-0 right-0 w-32 h-32 bg-orange-500/5 rounded-full blur-2xl pointer-events-none" />
              <div className="flex items-center gap-2 mb-3">
                <TrendingDown className="w-4 h-4 text-orange-400" />
                <span className="text-[11px] font-bold text-orange-400 uppercase tracking-widest font-mono">Routing Advantage</span>
              </div>
              <h5 className="text-[14px] font-semibold text-white tracking-tight leading-snug">
                Save up to 88% on costs by combining {selectedModel.name} with cheaper endpoints.
              </h5>
              <p className="text-xs text-gray-400 mt-2 leading-relaxed">
                Using BurnCloud's dynamic intent routing, easy queries are automatically directed to low-cost nodes, utilizing {selectedModel.name} only for high-complexity segments.
              </p>
            </div>

            <div className="pt-4">
              <Button onClick={() => setSelectedModel(null)} className="w-full">
                Close Model Profile
              </Button>
            </div>
          </div>
        )}
      </Drawer>
    </div>
  );
}

