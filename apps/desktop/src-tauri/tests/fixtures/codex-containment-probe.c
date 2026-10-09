#include <arpa/inet.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>

static int connects(int port) {
  int fd = socket(AF_INET, SOCK_STREAM, 0);
  struct sockaddr_in address = {0};
  address.sin_family = AF_INET;
  address.sin_port = htons(port);
  address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  int result = connect(fd, (struct sockaddr *)&address, sizeof(address));
  close(fd);
  return result == 0;
}
int main(int argc, char **argv) {
  if (argc != 6) return 2;
  int fd = open(argv[1], O_RDONLY);
  if (fd >= 0) { close(fd); puts("outside read allowed"); return 3; }
  fd = open(argv[2], O_WRONLY | O_CREAT, 0600);
  if (fd < 0) { puts("scratch write denied"); return 4; }
  close(fd);
  fd = open(argv[3], O_WRONLY | O_CREAT, 0600);
  if (fd >= 0) { close(fd); puts("outside write allowed"); return 5; }
  pid_t child = fork();
  if (child == 0) _exit(7);
  if (child > 0) { waitpid(child, NULL, 0); puts("fork allowed"); return 6; }
  if (!connects(atoi(argv[4]))) { puts("gateway denied"); return 8; }
  if (connects(atoi(argv[5]))) { puts("arbitrary loopback allowed"); return 9; }
  char *args[] = {"sh", "-c", "exit 0", NULL};
  execv("/bin/sh", args);
  puts("read/write/exec/fork/egress boundaries held");
  return 0;
}
