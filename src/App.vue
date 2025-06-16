<script setup lang="ts">
// --- Imports --- //
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { v4 as uuidv4 }from 'uuid'

// --- Types --- //
enum Header {
  Year,
  Days,
  Size,
  Name,
}

interface ProjectDef {
  selected: boolean
    id: string
  name: string
  year: number
  days: number
  size: number
}

enum ToastLevel {
  Message,
  Warn,
  Error,
}

interface Toast {
  id: string
  message: string
  severity: ToastLevel
  duration: number
}

// --- Constants --- //
const headers = [
  { label: "Revit Year", value: Header.Year },
  { label: "Age", value: Header.Days },
  { label: "Size on Disk", value: Header.Size },
  { label: "Folder Name", value: Header.Name }
]

// --- State --- //
const sortHeader = ref<Header>(Header.Days)
const sortOrder  = ref<boolean>(true) // For sortOrder, True is increasing (1, 2, 3) and False is decreasing.
const isLoading = ref<boolean>(false)
const projects = ref<ProjectDef[]>([
  { selected: false, name: "Loading...", year: 2021, days: 295, size: 250000, id: "project-folder-alpha" },
  { selected: false, name: "Placeholder...", year: 2025, days: 14, size: 500000, id: "project-folder-beta" },
  { selected: false, name: "Example...", year: 2023, days: 150, size: 0, id: "project-folder-gamma" }
]);
const selectDays = ref<number>(7)
const deletePayload = ref<ProjectDef[]>([])
const showConfirm = ref<boolean>(false)

const toastArray = ref<Toast[]>([])

// --- Getters --- //
const selectAll = computed<boolean>({
  get() {
    return projects.value.every(prj => prj.selected)
  },
  set(newValue) {
    for (const data of projects.value) {
      data.selected = newValue
    }
  }
})

const payloadSize = computed(() => deletePayload.value.length )

const sortedProjects = computed(() => {
  const sorted = [...projects.value]

  sorted.sort((a,b) => {
    let comparison = 0
    switch (sortHeader.value) {
      case Header.Year:
        comparison = a.year - b.year
        break;
      case Header.Days:
        comparison = a.days - b.days
        break;
      case Header.Size:
        comparison = a.size - b.size
        break;
      case Header.Name:
        comparison = a.name.localeCompare(b.name)
        break;
    }
    return sortOrder.value ? comparison : -comparison
  })

  return sorted
})

// --- Actions --- //
function addToast(message: string, severity: ToastLevel, duration: number = 2000) {
  const id = uuidv4()
  toastArray.value.push({ id, message, severity , duration})

  if (duration > 0) {
    setTimeout(() => {
      clearToast(id)
    }, duration)
  }
}

function clearToast(id: string) {
  toastArray.value = toastArray.value.filter(t => t.id !== id)
}

function styleSeverity(severity: ToastLevel) {
  switch (severity) {
    case ToastLevel.Message:
      return 'bg-slate-400/90'
    case ToastLevel.Warn:
      return 'bg-yellow-200/90'
    case ToastLevel.Error:
      return 'bg-red-300/90'
  }
}

function newSort(header: Header) {
  if (header === sortHeader.value) {
    sortOrder.value = !sortOrder.value
  } else {
    sortOrder.value = true
    sortHeader.value = header
  }
}

async function refreshData() {
  if (isLoading.value) {
    console.log('Already loading, please wait.')
    addToast("Already loading, please wait.", ToastLevel.Warn)
    return
  }
  console.log('Refreshing Data...')
  addToast("Refreshing Data...", ToastLevel.Message)
  isLoading.value = true

  try {

    const recievedProjects = await invoke('get_projects') as Omit<ProjectDef, 'selected'>[]

    projects.value = recievedProjects.map(prj => { return { selected: false, ...prj } })

  } catch (e) {
    console.error(e)
    addToast(String(e), ToastLevel.Error)
  }

  isLoading.value = false
  console.log('Data Refreshed!')
  addToast("Data Refreshed!", ToastLevel.Message)
}

function selectByDays() {
  for (const data of projects.value) {
    data.selected = data.days >= selectDays.value;
  }
}

function startDelete() {
  deletePayload.value = projects.value.filter(prj => prj.selected)
  showConfirm.value = true
}

async function confirmDelete() {
  if (isLoading.value) {
    console.log('Data is being loaded, please wait.')
    addToast("Data is being loaded, please wait.", ToastLevel.Warn)
    return
  }
  showConfirm.value = false
  console.log('Starting Deletion of files...')
  addToast("Starting Deletion of files...", ToastLevel.Message)
  isLoading.value = true

  const deletePromises = deletePayload.value.map(item => 
    invoke('delete_folder', { id: item.id }).catch(e => {
      console.error(e)
      addToast(String(e), ToastLevel.Error)
      return e
    })
  )

  await Promise.all(deletePromises)

  isLoading.value = false
  console.log('Deletion Complete!')
  addToast("Deletion Complete!", ToastLevel.Message)
  refreshData()  
}

async function openProject(project: ProjectDef) {
  const project_id = project.id
  console.log(`Attempting to open Project at ${project_id}`)

  try {
    await invoke('open_project', { id: project_id })
    console.log(`Project opened at ${project_id}`)    
    addToast(`Opening project folder...`, ToastLevel.Message)
  } catch(e) {
    console.error(e)
    addToast(String(e), ToastLevel.Error)
  }
}

async function openVers(project: ProjectDef) {
  const project_vers = project.year
  console.log(`Attempting to open collaboration cache folder for Revit version ${project_vers}`)

  try {
    await invoke('open_vers', { vers: project_vers })
    console.log(`Opening collaboration cache folder for Revit version ${project_vers}`)
    addToast(`Opening collaboration cache folder for Revit version ${project_vers}`, ToastLevel.Message)
  } catch(e) {
    console.error(e)
    addToast(String(e), ToastLevel.Error)
  }
}

function formatBytes(size: number) {
  if (size === 0) return '0 B'

  const k = 1024
  const decimals = 2
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']

  const index = Math.floor(Math.log(size) / Math.log(k))

  return parseFloat((size / Math.pow(k, index)).toFixed(decimals)) + ' ' + sizes[index]

}

// --- Setup --- //
onMounted(() => {
  refreshData()
})

</script>

<template>
  <div class="flex flex-col py-4 gap-2 text-zinc-50 h-screen relative overflow-hidden">

    <!-- Toasts -->
    <TransitionGroup name="toast" tag="div" class="absolute bottom-0 right-0 w-1/3 h-full z-15 pointer-events-none flex flex-col-reverse gap-2 p-2">
      <div v-for="toast in toastArray" :key="toast.id" 
        class="w-full px-3 py-1 rounded-2xl text-neutral-900 pointer-events-auto cursor-pointer"
        :class="styleSeverity(toast.severity)"
        @click="clearToast(toast.id)"
      >
        {{ toast.message }}
      </div>
      
    </TransitionGroup>

    <!-- Delete Confirmation -->
    <div v-if="showConfirm"
      class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2
             w-3/4 z-40 bg-slate-900 grid grid-cols-2 p-4 rounded-2xl border-white border-2 justify-items-center gap-x-2 gap-y-3 select-none">
        <div class="col-span-2 text-center">
          <p class="font-semibold text-xl pb-2"> You are about to delete {{ payloadSize }} folder{{ payloadSize === 1 ? "" : "s" }}.</p>
          <p>This cannot be undone, but these files will be re-downloaded when the project is loaded through Revit. This will not delete files from ACC.</p>
        </div>
        <button 
          type="button"
          class="px-2 py-1 text-center rounded-lg bg-slate-500 active:bg-slate-600 cursor-pointer select-none w-full"
          @click="confirmDelete"
          >
          Continue
        </button>
        <button 
          type="button"
          class="px-2 py-1 text-center rounded-lg bg-slate-700 active:bg-slate-800 cursor-pointer select-none w-full"
          @click="() => showConfirm = false"
        >Cancel
      </button>
      <div class="col-span-2 text-center">
          <p class="text-red-400 font-bold text-2xl">Warning!</p>
          <p class="text-red-400 font-semibold">Before you click continue, synchronize and save all work in Revit, then close all instances of Revit to prevent errors and corrupted data. Deleting files for an open project <i>will</i> cause issues.</p>
          <p>This program cannot detect if Revit is open or if any projects are loaded.</p>
        </div>
    </div>

    <!-- Loading Spinner -->
    <div v-if="isLoading" 
      class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2
             h-full w-full z-20
             justify-items-center items-center bg-zinc-200/15"
    >
      <svg
        class="animate-spin text-orange-500 h-1/2 translate-y-1/2"
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
      >
        <g>
          <circle
            class="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            stroke-width="4"
            fill="none"
          />
          <circle
            class="opacity-75"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            stroke-width="4"
            fill="none"
            :stroke-dasharray="Math.PI * 2 * 10"
            :stroke-dashoffset="Math.PI * 2 * 10 * (1 - 1/3) /* Adjust fraction here for spinner size */" 
          />
        </g>
      </svg>
    </div>

    <!-- Header and Info -->
    <div class="self-center text-2xl font-bold select-none">Welcome to the Collaboration Cache Cleaner!</div>

    <div class="text-sm px-8 select-none">
      <p>This tool is used to manage the folders in your Revit collaboration cache.</p>
      <p>Click on any Revit Year to open the collaboration cache folder for that year in your file explorer.</p>
      <p>Click on any Folder Name to open a the folder in your file explorer.</p>
    </div>

    <!-- Table -->
    <div class="flex-1 overflow-y-auto pl-4 scrollbar-stable scrollbar-color scrollbar-track-zinc-900 scrollbar-thumb-zinc-600 text-nowrap">
      <table class="min-w-full table-fixed">
        <thead class="bg-zinc-800 sticky top-0 z-10 select-none">
          <tr>
            <th class="px-3 py-2 w-auto text-center relative"> <input type="checkbox" class="cursor-pointer" v-model="selectAll"/> </th>

            <th 
              v-for="header of headers" 
              :key="header.label" 
              :class="['px-1 py-2 text-center relative cursor-pointer active:text-zinc-400', 
              (header.value === Header.Name ? 'w-full' : 'w-auto') ]"
              @click="newSort(header.value)"
            >
              <span class="inline-flex items-center gap-1">
              <span class="text-xs w-3 inline-block text-right">&nbsp;</span>
              {{ header.label }}
              <span v-if="sortHeader === header.value" class="text-xs w-3 inline-block text-right">
                {{ sortOrder ? '▲' : '▼' }}
              </span>
              <span v-else class="text-xs w-3 inline-block text-right">&nbsp;</span>
              </span>
            </th>

          </tr>
        </thead>
        <tbody>
          <tr v-for="item in sortedProjects" :key="item.id" class="select-none bg-zinc-600 even:bg-zinc-700">

            <td class="px-3 py-1 text-center"> <input type="checkbox" class="cursor-pointer" v-model="item.selected"/> </td>

            <td class="px-3 py-1 text-center cursor-pointer active:text-zinc-300"
              @click="openVers(item)"
            >{{ item.year }}</td>

            <td class="px-3 py-1 text-center">{{ item.days }} days</td>

            <td class="px-3 py-1 text-center">{{ formatBytes(item.size) }}</td>

            <td class="px-3 py-1 text-center cursor-pointer active:text-zinc-300"
              @click="openProject(item)"
            >{{ item.name }}</td>

          </tr>
        </tbody>
      </table>
    </div>

    <!-- Control Buttons -->
    <div class="flex flex-row gap-2 px-4 text-nowrap">
      <button
        type="button"
        class="px-2 py-1 text-center rounded-lg bg-slate-700 active:bg-slate-800 cursor-pointer select-none"
        @click="refreshData"
      >
        Refresh List
      </button>

      <button
        type="button"
        class="px-2 py-1 text-center rounded-lg bg-slate-700 active:bg-slate-800 has-[:focus]:bg-slate-700 cursor-pointer select-none flex gap-2"
        @click="selectByDays"
      >
        <span>Select all older than:</span>
        <input
          type="number"
          class="block w-8 border-0 focus:outline-none text-center rounded-sm bg-slate-500"
          @click.stop
          v-model="selectDays"
        >
        <span>Days</span>
      </button>

      <button
        type="button"
        class="px-2 py-1 text-center rounded-lg bg-slate-700 active:bg-slate-800 cursor-pointer select-none"
        @click="startDelete"
      >
        Delete Selected
      </button>

    </div>

  </div>
</template>

<style>

.toast-enter-active,
.toast-leave-active {
  transition: all 0.5s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(30px);
}

</style>