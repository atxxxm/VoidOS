#define _GNUC_SOURCE 

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <time.h>
#include <string.h>
#include <sys/wait.h>
#include <signal.h>

static volatile sig_atomic_t running = 1;

// SIGTERM and SIGINT handler
void sig_handler(int sig) {
    running = 0;
}

// field matching
int matches_field(int value, const char* failed) {
    // failed is "*"
    if (strcmp(failed, "*") == 0) return 1;

    // failed is a range
    return atoi(failed) == value;
}

// run command
void run_command(const char* cmd) {
    pid_t pid = fork();

    if (pid == 0) {
        execl(cmd, cmd, NULL);
        perror("execl failed");
        _exit(1);
    }
}

int main() {
    // setup signal handlers
    signal(SIGTERM, sig_handler);
    signal(SIGINT, sig_handler);
    
    printf("crond started\n");

    // read crond file
    FILE* cronfile = fopen("/etc/crontab", "r");

    if (!cronfile) {
        perror("cannot open /etc/crontab");
        return 1;
    }

    // parse crond file
    char line[256];

    // jobs
    struct {
        char min[16], hour[16], mday[16], mon[16], wday[16], cmd[128];
    } jobs[64];

    int job_count = 0;

    // read lines
    while (fgets(line, sizeof(line), cronfile)) {
        if (line[0] == '#' || line[0] == '\n') continue;

        if (sscanf(line, "%15s %15s %15s %15s %15s %127s[^\n]",
            jobs[job_count].min,
            jobs[job_count].hour,
            jobs[job_count].mday,
            jobs[job_count].mon,
            jobs[job_count].wday,
            jobs[job_count].cmd
        ) == 6) {
            job_count++;
        }
    }

    fclose(cronfile);

    // run jobs
    time_t last_minute = 0; 

    while (running) {
        time_t now = time(NULL);
        struct tm* tm = localtime(&now);

        // run jobs every minute
        if (tm->tm_min != last_minute) {
            last_minute = tm->tm_min;

            for (int i = 0; i < job_count; i++) {
                if (matches_field(tm->tm_min,     jobs[i].min)  &&
                    matches_field(tm->tm_hour,    jobs[i].hour) &&
                    matches_field(tm->tm_mday,    jobs[i].mday) &&
                    matches_field(tm->tm_mon + 1, jobs[i].mon)  &&
                    matches_field(tm->tm_wday,    jobs[i].wday)) 
                    {
                        printf("crond: running %s\n", jobs[i].cmd);
                        run_command(jobs[i].cmd);
                }
            }
        }

        sleep(1);
    }

    // cleanup
    printf("crond stopped\n");
    return 0;
}