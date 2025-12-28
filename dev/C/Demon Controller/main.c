#include "library/include/void_string.h"
#include "library/include/void_vec.h"
#include "library/include/void_file.h"
#include <stdio.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/types.h>

// Demons structure
typedef struct {
    VString name;
    VString path; 
    pid_t pid;
} Demons;

// Start a demon
void start_demon(Demons *d);
// Restart a demon
void restart_demon(Demons *d);
// Monitor demons
void monitor_demons(VVec *demons);

static volatile int running = 1;

// Handle SIGTERM
void handle_sigterm(int sig) {
    running = 0;
}

// Constants
#define DEMONS_FILE "/etc/demons.d"
#define DEMONS_DIR "/etc/demons/"

int main() {
    // Open the demons file
    VFile *vf = VFile_open(DEMONS_FILE, "r");

    // Check if the file was opened successfully
    if (!vf) {
        perror("Cannot open /etc/demons.d");
        return 1;
    }


    VString line = VString_new();
    VVec demons;
    vvec_init(&demons, sizeof(Demons));

    // Read the file line by line
    while(VFile_read_line(vf, &line)) {
        VString filename = VString_clone(&line);
        VString filepath = VString_from(DEMONS_DIR);
        VString_push_str(&filepath, VString_cstr(&filename));

        Demons d = {0};
        d.name = VString_clone(&filename);
        d.path = VString_clone(&filepath);
        d.pid = 0;
        vvec_push(&demons, &d);
        VString_clear_fast(&line);
    }

    VFile_close(vf);

    // Start demons
    for (size_t i = 0; i < demons.len; i++) {
        Demons *d = (Demons*)vvec_at(&demons, i);
        start_demon(d);
    }

    printf("Demon Controller: enterning main loop\n");

    signal(SIGTERM, handle_sigterm);

    // Main loop
    while (running)
    {
        // Monitor demons
        monitor_demons(&demons);
        sleep(1);
    }

    // Kill all demons
    for (size_t i = 0; i < demons.len; i++) {
        Demons *d = (Demons*)vvec_at(&demons, i);
        if (d->pid > 0) {
            kill(d->pid, SIGTERM);
            waitpid(d->pid, NULL, 0);
        }
    }

    // Free memory
    VString_free(&line);
    vvec_free(&demons);

    return 0;
}

void start_demon(Demons *d) {
    if (d == NULL) return;

    pid_t pid = fork();

    // Child process
    if (pid == 0) {
        execl(VString_cstr(&d->path), VString_cstr(&d->name), NULL);
        perror("execl failed");
        _exit(1);
    }
    else if (pid > 0) {
        // Parent process
        d->pid = pid;
        printf("Started demon: %s (pid=%d)\n", VString_cstr(&d->name), (int)pid);
    }
    else {
        perror("fork failed");
    }
}

void restart_demon(Demons *d) {
    if (d == NULL) {
        perror("Error path or name to restart");
        return;
    }

    sleep(1);
    start_demon(d);
}

void monitor_demons(VVec *demons) {
    int status;
    pid_t pid;

    // Wait for any child process to change state
    while ((pid = waitpid(-1, &status, WNOHANG)) > 0) {
        for (size_t i = 0; i < demons->len; i++) {
            Demons *d = (Demons*)vvec_at(demons, i);

            // Check if the process is the one we're looking for
            if (d->pid == pid) {
                printf("[%s] exited (status=%d)\n", VString_cstr(&d->name), status);

                // Restart the demon if it exited abnormally
                if (WIFSIGNALED(status) || WEXITSTATUS(status) != 0) {
                    printf("[%s] restarting...\n", VString_cstr(&d->name));
                    start_demon(d);
                }
                else {
                    // Mark the demon as exited normally
                    printf("[%s] exited normally, not restarting\n", VString_cstr(&d->name));
                    d->pid = 0;
                }
                break;
            }
        }
    }
}