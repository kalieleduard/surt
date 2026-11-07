import {
  ArrowDown,
  ArrowUp,
  Bell,
  CircleUserRound,
  Coins,
  DollarSign,
  Gem,
  House,
  Plus,
  Settings,
  ShipWheel,
  Wallet,
  X,
  type LucideIcon,
} from "lucide-react";
import "./reference-phone.css";

type Trend = "up" | "dip" | "down";
type CoinTone = "mineral" | "brasa" | "lava";

const ACTIONS: { label: string; icon: LucideIcon }[] = [
  { label: "Sand", icon: ArrowUp },
  { label: "Purchase", icon: Plus },
  { label: "Receive", icon: ArrowDown },
];

const TOKENS: {
  name: string;
  price: string;
  change: string;
  trend: Trend;
  amount: string;
  icon: LucideIcon;
  tone: CoinTone;
  active?: boolean;
}[] = [
  { name: "Ethereum", price: "$150.77", change: "+2.55%", trend: "up", amount: "0.2058679", icon: Gem, tone: "mineral" },
  { name: "Bitcoin", price: "$50.51", change: "-7.85%", trend: "dip", amount: "0.0158679", icon: Coins, tone: "brasa", active: true },
  { name: "USD", price: "$150.01", change: "-12.80%", trend: "down", amount: "0.6058679", icon: DollarSign, tone: "lava" },
];

const NAV: { label: string; icon: LucideIcon; active?: boolean }[] = [
  { label: "Home", icon: House, active: true },
  { label: "Wallet", icon: Wallet },
  { label: "Profile", icon: CircleUserRound },
];

function TokenRow({ token }: { token: (typeof TOKENS)[number] }) {
  const Icon = token.icon;
  return (
    <div className="ref-coin" data-active={token.active}>
      <span className="ref-coin-logo" data-tone={token.tone}>
        <Icon size={15} />
      </span>
      <span className="ref-coin-text">
        <span className="ref-coin-name">{token.name}</span>
        <span className="ref-coin-sub">
          {token.price}
          <span className="ref-coin-change" data-trend={token.trend}>
            {token.change}
          </span>
        </span>
      </span>
      <span className="ref-coin-amount">{token.amount}</span>
    </div>
  );
}

export function ReferencePhone() {
  return (
    <div className="ref-phone">
      <div className="ref-phone-bar">
        <span />
        <span />
        <span />
        <Settings size={11} className="ref-phone-gear" />
      </div>
      <div className="ref-phone-top">
        <Bell size={15} />
        <div className="ref-seg" role="tablist">
          <button type="button" role="tab" aria-selected="true">
            Tokens
          </button>
          <button type="button" role="tab" aria-selected="false">
            Collection
          </button>
        </div>
        <ShipWheel size={15} />
      </div>
      <span className="ref-phone-glow" />
      <p className="ref-phone-caption">Current wallet value</p>
      <p className="ref-phone-amount">$60,567.12</p>
      <p className="ref-phone-change">+$201,00 (1.5%)</p>
      <div className="ref-phone-actions">
        {ACTIONS.map(({ label, icon: Icon }) => (
          <button type="button" key={label} className="ref-phone-action">
            <span className="ref-phone-action-dot">
              <Icon size={17} strokeWidth={2} />
            </span>
            {label}
          </button>
        ))}
      </div>
      <section className="ref-sheet">
        <span className="ref-sheet-handle" />
        <header className="ref-sheet-title">
          Portfolio
          <button type="button" className="ref-sheet-close" aria-label="Close">
            <X size={13} />
          </button>
        </header>
        {TOKENS.map((token) => (
          <TokenRow key={token.name} token={token} />
        ))}
        <nav className="ref-phone-nav">
          {NAV.map(({ label, icon: Icon, active }) => (
            <span key={label} className="ref-phone-tab" data-active={active}>
              <Icon size={20} strokeWidth={1.7} />
              {label}
            </span>
          ))}
        </nav>
        <span className="ref-phone-home" />
      </section>
    </div>
  );
}
