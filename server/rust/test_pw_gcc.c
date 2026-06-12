#include <pipewire/pipewire.h>
int main() {
    pw_init(NULL, NULL);
    pw_deinit();
    return 0;
}
