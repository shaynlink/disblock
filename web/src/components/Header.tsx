import { Link } from "react-router";
import { avatarUrl } from "../api.ts";
import type { User } from "../types.ts";

export function Logo() {
  return (
    <span className="logo">
      <img src="/favicon.svg" alt="" width={24} height={24} />
      Disblock
    </span>
  );
}

export function Header({ user, onLogout }: { user: User; onLogout: () => void }) {
  return (
    <header className="header">
      <Link to="/" className="header-home">
        <Logo />
      </Link>
      <div className="header-user">
        <img
          className="avatar"
          src={avatarUrl(user.discord_id, user.avatar)}
          alt=""
          width={28}
          height={28}
        />
        <span>{user.username}</span>
        <button type="button" className="button ghost" onClick={onLogout}>
          Sign out
        </button>
      </div>
    </header>
  );
}
