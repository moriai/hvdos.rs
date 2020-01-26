#include "DOSKernel.h"
#include <Hypervisor/hv.h>

extern "C" {
    typedef struct {
        DOSKernel inst;
    } DOSKernelInst;

    DOSKernelInst* DOSKernel_DOSKernel(char *memory, hv_vcpuid_t vcpu, int argc, char **argv) {
        DOSKernel *inst = new DOSKernel(memory, vcpu, argc, argv);
        return (DOSKernelInst*)inst;
    }

    void DOSKernel_DOSKernel_destructor(DOSKernelInst* ki) {
        ki->inst.~DOSKernel();
    }

    int DOSKernel_dispatch(DOSKernelInst* ki, uint8_t IntNo) {
        return ki->inst.dispatch(IntNo);
    }

    int DOSKernel_exitStatus(DOSKernelInst* ki) {
        return ki->inst.exitStatus();
    }
}

/* read GPR */
uint64_t
rreg(hv_vcpuid_t vcpu, hv_x86_reg_t reg)
{
	uint64_t v;

	if (hv_vcpu_read_register(vcpu, reg, &v)) {
		abort();
	}

	return v;
}

/* write GPR */
void
wreg(hv_vcpuid_t vcpu, hv_x86_reg_t reg, uint64_t v)
{
	if (hv_vcpu_write_register(vcpu, reg, v)) {
		abort();
	}
}
