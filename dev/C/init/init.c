#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <string.h>
#include <time.h>
#include <errno.h>
#include <stdarg.h>

// Start demons
void start_demons() {

}

int create_dir(const char *path);

int main() {
    // Output to console
    freopen("/dev/console", "r", stdin);
    freopen("/dev/console", "w", stdout);
    freopen("/dev/console", "w", stderr);

    printf("VoidOS init starting...\n");

    printf("Start mounting...\n");

    // Mount filesystems
    mount("proc", "/proc", "proc", 0, "");
    mount("sysfs", "/sys", "sysfs", 0, "");
    mount("tmpfs", "/tmp", "tmpfs", 0, "");
    mount("devtmpfs", "/dev", "devtmpfs", 0, "");

    // Set PATH
    setenv("PATH", "/bin:/sbin", 1);

    // Create home directory if it doesn't exist
    create_dir("home");

    printf("Initialization complete. Starting shell...\n");

    // Main loop
    while (1) {
        int status;
        pid_t pid = waitpid(-1, &status, WNOHANG);

        if (pid > 0) {
            printf("[init] demon exited unexpectedly (pid: %d, status: %d)\n", pid, status);
            start_demons();
        }

        pid_t shell_pid = fork();
        if (shell_pid == 0) {
            execl("/bin/vsh", "vsh", NULL);
            perror("Failed to start vsh");
            _exit(1);
        }
        else if (shell_pid > 0) {
            waitpid(shell_pid, NULL, 0);
            printf("Shell exited. Restarting...\n");
        }
        else {
            perror("Failed to fork");
            sleep(2);
        }
    }

    return 0;
}

int create_dir(const char *path) {
    struct stat st;

    if (stat(path, &st) == 0) {
        if (S_ISDIR(st.st_mode)) {
            return 0;
        }
    }

    if (mkdir(path, 0755) == 0) {
        printf("Created directory: %s\n", path);
    }
}