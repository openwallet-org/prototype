//
// Jump to the application entry point.
//
// NOTE: This function sets the MCU's MSP and PSP (stack pointer) registers to the initial
//      stack pointer value as found in the application's vector table. It also sets the
//      MCU's VTOR (Vector Table Offset Register) to point to the application's vector
//      table.
//
void __attribute__((noreturn)) blJumpToApplication()
{
    // We need static variables to hold the application address and initial stack pointer
    // since changing the stack pointer register will invalidate any automatic variables
    static uint32_t stackpointer = 0;
    static void (*appentrypoint)(void) = 0;

    // Disable interrupts at Task level
    taskDISABLE_INTERRUPTS();

    // Disable interrupts at the CPU level
    __disable_irq();

    // Disable all interrupts in the NVIC
    NVIC->ICER[0U] = 0xFFFFFFFFU;
    NVIC->ICER[1U] = 0xFFFFFFFFU;
    NVIC->ICER[2U] = 0xFFFFFFFFU;
    NVIC->ICER[3U] = 0xFFFFFFFFU;
    NVIC->ICER[4U] = 0xFFFFFFFFU;
    NVIC->ICER[5U] = 0xFFFFFFFFU;
    NVIC->ICER[6U] = 0xFFFFFFFFU;

    // Per STM PM0214 Rev 7, "Bits 16 to 31 of the NVIC_ICER7 register are reserved."
    NVIC->ICER[7U] = 0x0000FFFFU;

    // Disable the SysTick timer
    SysTick->CTRL = 0;

    // Get the initial SP and the app entry point from the application's vector table
    // The initial SP is word 0, the application entry point is word 1
    uint32_t *APP_VECTOR_TABLE = (uint32_t *)APPLICATION_BASE;
    stackpointer = APP_VECTOR_TABLE[0];
    appentrypoint = (void (*)(void))APP_VECTOR_TABLE[1];

    // Point the MCU's vector table offset register to the application's vector table
    SCB->VTOR = APPLICATION_BASE;

    // Set the MCU's stack pointer registers
    __set_MSP(stackpointer);
    __set_PSP(stackpointer);

    // Jump to the application's entry point
    appentrypoint();

    // Quiet the compiler warning about returning from a function marked as noreturn
    for (;;)
    {
        // Endless do nothing loop
    }
}