#define _GNU_SOURCE

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <signal.h>
#include <string.h>
#include <time.h>
#include <errno.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/stat.h>
#include <fcntl.h>

// Constants
#define SOCKET_PATH "/dev/log"
#define LOG_FILE "/var/log/messages"

static volatile sig_atomic_t running = 1;

// Signal handler for SIGTERM and SIGINT
void sigterm_handler(int sig) {
    running = 0;
}

// Log to file
void log_to_file(const char* msg) {
    // Open log file
    FILE* f = fopen(LOG_FILE, "a");

    if (!f) return;

    // Get current time
    time_t now = time(NULL);
    struct tm* tm = localtime(&now);
    char time_str[32];
    strftime(time_str, sizeof(time_str), "%b %d %H:%M:%S", tm);

    // Write to log file
    fprintf(f, "%s %s\n", time_str, msg);
    fclose(f);
}

int main() {
    // Handle signals
    signal(SIGTERM, sigterm_handler);
    signal(SIGINT, sigterm_handler);

    // Create log directory and socket
    mkdir("/var/log", 0755);
    unlink(SOCKET_PATH);

    // Create socket
    int sock = socket(AF_UNIX, SOCK_DGRAM, 0);
    if (sock < 0) {
        perror("socket");
        return 1;
    }

    // Bind socket
    struct sockaddr_un addr = {0};
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, SOCKET_PATH, sizeof(addr.sun_path) - 1);

    if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        perror("bind");
        close(sock);
        return 1;
    }

    // Set permissions
    chmod(SOCKET_PATH, 0666);

    printf("syslogd started, listening on %s\n", SOCKET_PATH);

    char buffer[1024];

    while (running) {
        // Receive message
        ssize_t len = recv(sock, buffer, sizeof(buffer) - 1, 0);

        // Log message
        if (len > 0) {
            buffer[len] = '\0';
            if (len > 0 && buffer[len - 1] == '\n') {
                buffer[len - 1] = '\0';
            }

            log_to_file(buffer);
        }
    }

    // Cleanup
    close(sock);
    unlink(SOCKET_PATH);
    printf("syslogd stopped\n");
    return 0;
}