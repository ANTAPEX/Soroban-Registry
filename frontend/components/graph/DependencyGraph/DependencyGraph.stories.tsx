import type { Meta, StoryObj } from '@storybook/react';
import DependencyGraph from './DependencyGraph';

const meta: Meta<typeof DependencyGraph> = {
  title: 'Components/DependencyGraph',
  component: DependencyGraph,
};

export default meta;
type Story = StoryObj<typeof DependencyGraph>;

export const Default: Story = {
  args: {
    nodes: [
      { id: '1', name: 'Contract A', network: 'mainnet' },
      { id: '2', name: 'Contract B', network: 'testnet' },
    ],
    edges: [
      { source: '1', target: '2', type: 'calls' },
    ],
  },
};
