#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/proc_fs.h>
#include <linux/sched.h>
#include <linux/slab.h>
#include <linux/uaccess.h>

#define BUFFER_SIZE 128
#define PROC_NAME "pid"

/* the current pid */
static long l_pid;

/* Function prototypes */
static ssize_t proc_read(struct file *file, char __user *usr_buf, size_t count, loff_t *pos);
static ssize_t proc_write(struct file *file, const char __user *usr_buf, size_t count, loff_t *pos);

/* File operations structure */
static struct file_operations proc_ops = {
    .owner = THIS_MODULE,
    .read = proc_read,
    .write = proc_write,
};

/* Called when the module is loaded */
static int proc_init(void)
{
    /* Creates /proc/pid entry */
    proc_create(PROC_NAME, 0666, NULL, &proc_ops);
    printk(KERN_INFO "/proc/%s created\n", PROC_NAME);
    return 0;
}

/* Called when the module is removed */
static void proc_exit(void)
{
    /* Removes /proc/pid entry */
    remove_proc_entry(PROC_NAME, NULL);
    printk(KERN_INFO "/proc/%s removed\n", PROC_NAME);
}

/* Called when /proc/pid is read */
static ssize_t proc_read(struct file *file, char __user *usr_buf, size_t count, loff_t *pos)
{
    struct task_struct *task;
    char buffer[BUFFER_SIZE];
    int len = 0;
    static int completed = 0;

    if (completed) {
        completed = 0;
        return 0;
    }

    completed = 1;

    /* Obtém a estrutura task_struct associada ao PID */
    task = pid_task(find_vpid(l_pid), PIDTYPE_PID);

    if (task == NULL) {
        len = snprintf(buffer, BUFFER_SIZE, "Invalid PID\n");
    } else {
        /* Formata as informações: comando, PID e estado */
        len = snprintf(buffer, BUFFER_SIZE,
                       "command = [%s] pid = [%d] state = [%ld]\n",
                       task->comm, task->pid, task->state);
    }

    /* Copia o buffer para o espaço do usuário */
    if (copy_to_user(usr_buf, buffer, len)) {
        return -EFAULT;
    }

    return len;
}

/* Called when /proc/pid is written */
static ssize_t proc_write(struct file *file, const char __user *usr_buf, size_t count, loff_t *pos)
{
    char *k_mem;

    /* Aloca memória no kernel */
    k_mem = kmalloc(count + 1, GFP_KERNEL);
    if (!k_mem) {
        return -ENOMEM;
    }

    /* Copia dados do espaço do usuário para o kernel */
    if (copy_from_user(k_mem, usr_buf, count)) {
        kfree(k_mem);
        return -EFAULT;
    }

    /* Adiciona o terminador nulo à string */
    k_mem[count] = '\0';

    /* Converte o valor para long */
    if (kstrtol(k_mem, 10, &l_pid) != 0) {
        printk(KERN_INFO "Invalid PID value\n");
        kfree(k_mem);
        return -EINVAL;
    }

    kfree(k_mem);
    printk(KERN_INFO "PID set to %ld\n", l_pid);
    return count;
}

/* Registra os pontos de entrada e saída do módulo */
module_init(proc_init);
module_exit(proc_exit);

MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Kernel module to display task info based on PID");
MODULE_AUTHOR("Adapted Example");
