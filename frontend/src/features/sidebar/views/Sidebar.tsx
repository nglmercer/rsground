import { setIsAuthOpen } from "@features/auth/stores";
import { AuthModal, RawUserAvatar } from "@features/auth/views";
import { setIsColabOpen } from "@features/colab/stores";
import { Colab } from "@features/colab/views";
import { FileExplorer } from "@features/file-explorer/views";
import { ContextMenu } from "@features/context-menu/views";
import { ThemeSelector } from "@features/theme/views";
import { MenuIcon } from "@icons/Menu";
import { ChevronLeftIcon } from "@icons/ChevronLeft";
import { DocumentIcon } from "@icons/Document";
import { ShareNodesIcon } from "@icons/ShareNodes";

import { isSidebarOpen, setIsSidebarOpen } from "../stores";
import { SidebarNavItem } from "./SidebarNavItem";

import styles from "./Sidebar.module.sass";

export function Sidebar() {
  return (
    <div
      class={styles.container}
      attr:data-closed={!isSidebarOpen() || null}
    >
      <nav class={styles.nav} aria-label="Workspace navigation">
        <ul class={styles.nav_items}>
          <ContextMenu
            as={SidebarNavItem}
            tooltip="Menu"
            useLeftClick
            useRightClick={false}
            followCursor={false}
            options={{
              Theme: <ThemeSelector />,
            }}
          >
            <MenuIcon aria-hidden="true" />
          </ContextMenu>

          <SidebarNavItem
            fullSized
            tooltip="Auth"
            onClick={() => setIsAuthOpen(true)}
          >
            <AuthModal>
              <RawUserAvatar />
            </AuthModal>
          </SidebarNavItem>

          <SidebarNavItem tooltip="Colab" onClick={() => setIsColabOpen(true)}>
            <ShareNodesIcon aria-hidden="true" />
            <Colab />
          </SidebarNavItem>

          <SidebarNavItem
            tooltip="Files"
            onClick={() => setIsSidebarOpen(true)}
          >
            <DocumentIcon aria-hidden="true" />
          </SidebarNavItem>

          <SidebarNavItem
            tooltip={isSidebarOpen() ? "Close" : "Open"}
            onClick={() => setIsSidebarOpen((prev) => !prev)}
          >
            <ChevronLeftIcon aria-hidden="true" />
          </SidebarNavItem>
        </ul>
      </nav>

      <div
        class={styles.body}
        attr:aria-hidden={!isSidebarOpen() || null}
      >
        <FileExplorer />
      </div>
    </div>
  );
}
